defmodule ExCompileGraph do
  @moduledoc """
  Documentation for `ExCompileGraph`.
  """

  @type manifest_path :: binary()
  @type file_path :: binary()

  def get_graph() do
    manifest = Mix.Project.manifest_path() <> "/compile.elixir"
    graph = __MODULE__.Graph.build(manifest)

    graph
    |> __MODULE__.Graph.summarize()
    |> Enum.map(fn vertex ->
      recompile = __MODULE__.Dependency.recompile_dependencies(graph, vertex.id)

      # These sets should be non-overlapping
      recompile_dependencies =
        Enum.reduce(recompile, [], fn {reason, sources}, acc ->
          MapSet.to_list(sources)
          |> Enum.map(fn file -> %{id: file, reason: reason} end)
          |> Enum.concat(acc)
        end)

      Map.put(vertex, :recompile_dependencies, recompile_dependencies)
    end)
  end
end
