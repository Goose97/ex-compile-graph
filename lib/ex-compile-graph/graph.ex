defmodule ExCompileGraph.Graph do
  @moduledoc """
  Represents dependencies between source files as a labeled directed graph
  """

  alias ExCompileGraph.Manifest
  @fixtures "lib/fixtures"

  @spec build(manifest :: binary()) :: :digraph.graph()
  def build(manifest) do
    graph = :digraph.new()
    manifest_lookup = Manifest.build_lookup_table(manifest)
    sourceFiles = Manifest.all_source_files(manifest_lookup)

    Enum.each(sourceFiles, fn sourceFile ->
      :digraph.add_vertex(graph, sourceFile.path)
    end)

    Enum.each(sourceFiles, fn sourceFile ->
      :digraph.add_vertex(graph, sourceFile.path)

      Enum.each(
        sourceFile.compile_references,
        fn module ->
          with {:ok, %{source_paths: source_paths}} <-
                 Manifest.lookup_module(manifest_lookup, module) do
            Enum.each(source_paths, &:digraph.add_edge(graph, sourceFile.path, &1, :compile))
          end
        end
      )

      Enum.each(
        sourceFile.export_references,
        fn module ->
          with {:ok, %{source_paths: source_paths}} <-
                 Manifest.lookup_module(manifest_lookup, module) do
            Enum.each(source_paths, &:digraph.add_edge(graph, sourceFile.path, &1, :exports))
          end
        end
      )

      Enum.each(
        sourceFile.runtime_references,
        fn module ->
          with {:ok, %{source_paths: source_paths}} <-
                 Manifest.lookup_module(manifest_lookup, module) do
            Enum.each(source_paths, &:digraph.add_edge(graph, sourceFile.path, &1, :runtime))
          end
        end
      )
    end)

    graph
  end

  defdelegate vertices(graph), to: :digraph
end
