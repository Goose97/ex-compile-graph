defmodule ExCompileGraph.Dependency do
  @moduledoc """
  Module contains API to answer queries based on the dependencies graph

  ## Recompile dependencies
  Given two files A and B, we state that A has a recompile dependency to B iff when B
  recompiles, A must recompiles as well.

  Recompile dependencies can be formed in these scenarios:
  1. A has a compile dependency to B
  2. A has a compile-then-runtime dependency to B (A has a compile dependency to A1 and A1
  has a runtime dependency to B)
  3. A has a exports dependency to B
  4. A has a exports-then-compile dependency to B (A has a exports dependency to A1 and A1
  has a compile dependency to B)

  There are two types of recompile dependencies, definite and indefinite.

  - Definite dependencies mean if the target file recompiles, the source file will DEFINITELY
  recompile. Scenario 1 and 2 above fell into this type.
  - Indefinite dependencies mean if the target file recompiles, the source file MAY or MAY NOT
  recompile. This is because exports dependencies depend on modules API, namely structs and
  public functions. So when a target file recompiles, but its struct definitions and public
  functions don't change, the source file won't recompile. Scenario 3 and 4 above fell into this type.
  """

  alias ExCompileGraph.{SourceFile, Manifest, SourceParser}
  @type dependency_type :: :compile | :exports | :runtime

  @doc """
  Returns all files which have a recompile dependency to the target file
  """
  @spec recompile_dependencies(:digraph.graph(), ExCompileGraph.file_path()) :: %{
          compile: MapSet.t(),
          exports_then_compile: MapSet.t(),
          exports: MapSet.t(),
          compile_then_runtime: MapSet.t()
        }
  def recompile_dependencies(graph, target_file) do
    compile_sources = find_source_files(graph, target_file, :compile)

    exports_then_compile_sources =
      Enum.reduce(compile_sources, MapSet.new(), fn file, acc ->
        find_source_files(graph, file, :exports, direct_only?: true)
        |> MapSet.union(acc)
      end)

    exports_sources = find_source_files(graph, target_file, :exports, direct_only?: true)

    compile_then_runtime_sources =
      find_source_files(graph, target_file, :runtime)
      |> Enum.reduce(MapSet.new(), fn file, acc ->
        find_source_files(graph, file, :compile)
        |> MapSet.union(acc)
      end)

    %{
      compile: compile_sources,
      exports_then_compile: exports_then_compile_sources,
      exports: exports_sources,
      compile_then_runtime: compile_then_runtime_sources
    }
  end

  @spec find_source_files(:digraph.graph(), binary(), dependency_type, direct_only?: boolean) ::
          MapSet.t()
  def find_source_files(graph, sink_file, dependency_type, opts \\ []),
    do: find_source_files(graph, sink_file, dependency_type, {MapSet.new(), sink_file}, opts)

  defp find_source_files(graph, sink_file, dependency_type, state, opts) do
    {result, initial_vertex} = state
    direct_only? = Keyword.get(opts, :direct_only?, false)

    source_files =
      :digraph.in_edges(graph, sink_file)
      |> Enum.flat_map(fn edge ->
        case :digraph.edge(graph, edge) do
          {_, source, _, ^dependency_type} ->
            # Ignore visited vertex and our inital vertex. Otherwise, we go into a infinite loop
            if MapSet.member?(result, source) or source == initial_vertex,
              do: [],
              else: [source]

          _ ->
            []
        end
      end)

    result = Enum.reduce(source_files, result, &MapSet.put(&2, &1))

    if direct_only?,
      do: result,
      else:
        Enum.reduce(
          source_files,
          result,
          &find_source_files(graph, &1, dependency_type, {&2, initial_vertex}, opts)
        )
  end

  @doc """
  Given two files and their dependency type, return all the causes for such dependency
  """
  @type dependency_causes_params :: %{
          source_file: binary(),
          sink_file: binary(),
          manifest: binary(),
          dependency_type: ExCompileGraph.dependency_type()
        }
  @spec dependency_causes(dependency_causes_params()) :: [
          __MODULE__.Cause.t()
        ]
  # There are 2 sources of exports dependency causes: 1) import and 2) struct usage
  def dependency_causes(%{dependency_type: :exports} = params) do
    %{
      source_file: source_file,
      sink_file: sink_file,
      manifest: manifest
    } = params

    file_lookup_table = ExCompileGraph.SourceFile.build_lookup_table(manifest)
    manifest_lookup_table = Manifest.build_lookup_table(manifest)
    %{modules: modules} = SourceFile.lookup!(file_lookup_table, source_file)

    absolute_source_file =
      if params[:root_folder],
        do: Path.join([params[:root_folder], source_file]),
        else: source_file

    absolute_sink_file =
      if params[:root_folder],
        do: Path.join([params[:root_folder], sink_file]),
        else: sink_file

    import_exprs =
      Enum.flat_map(modules, fn module ->
        exprs = SourceParser.import_exprs(absolute_source_file, module)

        Enum.flat_map(exprs, fn expr ->
          target_module = SourceParser.import_target(expr)

          case Manifest.lookup_module(manifest_lookup_table, target_module) do
            {:ok, %{source_paths: source_paths}} ->
              [{expr, source_paths}]

            _ ->
              []
          end
        end)
      end)

    import_causes =
      Enum.flat_map(import_exprs, fn
        {import_expr, sink_files} ->
          if sink_file in sink_files,
            do: [
              %__MODULE__.Cause{
                name: :import,
                origin_file: source_file,
                lines_span: SourceParser.expr_lines_span(import_expr)
              }
            ],
            else: []

        _ ->
          []
      end)

    struct_defs =
      for {struct_name, _} <- SourceParser.struct_defs!(absolute_sink_file), do: struct_name

    struct_usage_causes =
      for struct_usage <- SourceParser.struct_expr(absolute_source_file, struct_defs) do
        %__MODULE__.Cause{
          name: :struct_usage,
          origin_file: source_file,
          lines_span: SourceParser.expr_lines_span(struct_usage)
        }
      end

    all_causes = import_causes ++ struct_usage_causes

    if all_causes != [],
      do: Enum.sort_by(all_causes, &{&1.origin_file, &1.lines_span}),
      else:
        raise(RuntimeError,
          message: """
          #{__MODULE__}.dependency_causes: can't find exports dependency cause between two files. This should not happen and is a bug in the implementation:
          - source_file: #{source_file}
          - sink_file: #{sink_file}
          """
        )
  end
end
