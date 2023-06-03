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

    # Cache for later use
    :ets.insert(__MODULE__.Cache, {:graph, graph})

    graph
    |> __MODULE__.Graph.summarize()
    |> Enum.map(fn vertex ->
      recompile = __MODULE__.Dependency.recompile_dependencies(graph, vertex.id)

      # These sets should be non-overlapping
      recompile_dependencies =
        Enum.reduce(recompile, [], fn {reason, dependency}, acc ->
          dependency
          |> Enum.map(fn {file, path} ->
            :ets.insert(__MODULE__.Cache, {{:dependency_path, vertex.id, file, reason}, path})
            %{id: file, reason: reason}
          end)
          |> Enum.concat(acc)
        end)

      Map.put(vertex, :recompile_dependencies, recompile_dependencies)
    end)
  end

  def get_recompile_dependency_causes(source_file, sink_file, reason) do
    [{_, path}] =
      :ets.lookup(__MODULE__.Cache, {:dependency_path, sink_file, source_file, reason})

    get_detailed_explanation(path ++ [{:eof, sink_file}])
    |> IO.inspect(label: inspect({source_file, sink_file, reason}))
  end

  # Expand the dependency path to a detailed explanation
  # Consecutive runtime links could be collapse into one node
  defp get_detailed_explanation(path) do
    initial_state = %{consecutive_runtime_links: [], result: [], prev: nil}
    get_detailed_explanation(path, initial_state)
  end

  defp get_detailed_explanation([], state), do: state.result

  defp get_detailed_explanation([{:runtime, file} | tail], state) do
    state =
      Map.update!(state, :consecutive_runtime_links, &(&1 ++ [file]))
      |> Map.put(:prev, file)

    get_detailed_explanation(tail, state)
  end

  defp get_detailed_explanation([{type, file} | tail], state)
       when type in [:exports, :compile, :eof] do
    state =
      if state.consecutive_runtime_links != [] do
        new_entry = %{
          type: :runtime,
          source: hd(state.consecutive_runtime_links),
          intermediates: tl(state.consecutive_runtime_links),
          snippets: nil
        }

        Map.update!(state, :result, &(&1 ++ [new_entry]))
        |> Map.put(:consecutive_runtime_links, [])
      else
        state
      end

    state =
      if type != :eof do
        manifest = Mix.Project.manifest_path() <> "/compile.elixir"
        # It's guarantee that we always have a next file here
        {_, next_file} = hd(tail)

        snippets =
          ExCompileGraph.Dependency.dependency_causes(%{
            source_file: file,
            sink_file: next_file,
            manifest: manifest,
            dependency_type: type
          })
          |> Enum.map(&extract_snippet/1)

        new_entry = %{
          type: type,
          source: file,
          intermediates: [],
          snippets: snippets
        }

        Map.update!(state, :result, &(&1 ++ [new_entry]))
      else
        state
      end

    state = Map.put(state, :prev, file)
    get_detailed_explanation(tail, state)
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
