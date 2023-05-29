defmodule ExCompileGraph do
  @moduledoc """
  Module contains API to handle Web UI interactions
  """

  @type manifest_path :: binary()
  @type file_path :: binary()

  def init() do
    :ets.new(__MODULE__.Cache, [:set, :named_table, :public])
    :ok
  end

  def get_graph() do
    manifest = Mix.Project.manifest_path() <> "/compile.elixir"
    graph = __MODULE__.Graph.build(manifest)

    :ets.insert(__MODULE__.Cache, {:graph, graph})

    graph_summary =
      for vertex <- __MODULE__.Graph.summarize(graph) do
        dependencies =
          __MODULE__.Dependency.recompile_dependencies(graph, vertex.id)
          |> Enum.flat_map(fn {reason, dependents} ->
            for {file, chain} <- dependents do
              %{
                id: "#{file}_#{reason}",
                path: file,
                reason: reason,
                dependency_chain: format_dependency_chain(chain, vertex.id)
              }
            end
          end)
          |> Enum.sort_by(& &1.id)

        Map.put(vertex, :recompile_dependencies, dependencies)
      end

    spawn(fn -> cache_dependency_path(graph_summary) end)

    graph_summary
  end

  # From this:
  # chain = [
  #   [:compile, "lib/fixtures/D1.ex"],
  #   [:compile, "lib/fixtures/D2.ex"]
  # ]
  # recompile_source = "lib/fixtures/D3.ex"
  #
  # To this:
  # chain = [
  #   [:compile, "lib/fixtures/D1.ex", "lib/fixtures/D2.ex"],
  #   [:compile, "lib/fixtures/D2.ex",  "lib/fixtures/D3.ex"]
  # ]
  defp format_dependency_chain([], _), do: []

  defp format_dependency_chain(chain, recompile_source) do
    Enum.zip(chain, tl(chain) ++ [{nil, recompile_source}])
    |> Enum.map(fn {item, {_, next_file}} ->
      Tuple.append(item, next_file)
    end)
  end

  defp cache_dependency_path(graph_summary) do
    Enum.each(graph_summary, fn vertex ->
      Enum.each(vertex.recompile_dependencies, fn dependent ->
        key = {:dependency_path, vertex.id, dependent.path, dependent.reason}
        :ets.insert(__MODULE__.Cache, {key, dependent.dependency_chain})
      end)
    end)
  end

  @spec get_recompile_dependency_causes(
          file_path,
          file_path,
          __MODULE__.Dependency.dependency_reason()
        ) :: any()
  def get_recompile_dependency_causes(source_file, sink_file, reason) do
    case :ets.lookup(__MODULE__.Cache, {:dependency_path, sink_file, source_file, reason}) do
      [{_, path}] ->
        get_detailed_explanation(path ++ [{:eof, sink_file, nil}])

      [] ->
        []
    end
  end

  # Expand the dependency path to a detailed explanation
  defp get_detailed_explanation(path) do
    get_detailed_explanation(path, [])
  end

  defp get_detailed_explanation([], result), do: result

  defp get_detailed_explanation([{:runtime, source, sink} | tail], result) do
    new_entry = %{
      type: :runtime,
      source: source,
      sink: sink,
      snippets: []
    }

    get_detailed_explanation(tail, result ++ [new_entry])
  end

  defp get_detailed_explanation([{type, source, sink} | tail], result)
       when type in [:exports, :compile, :eof] do
    result =
      if type != :eof do
        manifest = Mix.Project.manifest_path() <> "/compile.elixir"

        snippets =
          ExCompileGraph.Dependency.dependency_causes(%{
            source_file: source,
            sink_file: sink,
            manifest: manifest,
            dependency_type: type
          })
          |> Enum.map(&extract_snippet/1)

        new_entry = %{
          type: type,
          source: source,
          sink: sink,
          snippets: snippets
        }

        result ++ [new_entry]
      else
        result
      end

    get_detailed_explanation(tail, result)
  end

  @lines_span_padding 5
  defp extract_snippet(%ExCompileGraph.Dependency.Cause{origin_file: file, lines_span: lines_span}) do
    # We want to add some paddings to the lines span
    {from, to} = lines_span
    from = max(from - @lines_span_padding, 1)
    to = to + @lines_span_padding

    lines =
      File.stream!(file, [], :line)
      |> Stream.drop(from - 1)
      |> Stream.take(to - from + 1)
      |> Enum.to_list()

    # to may exceeds the file lines count
    to = min(from + length(lines) - 1, to)

    %{
      content: Enum.join(lines),
      lines_span: [from, to],
      highlight: Tuple.to_list(lines_span)
    }
  end
end
