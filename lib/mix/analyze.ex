defmodule Mix.Tasks.ExCompileGraph.Server do
  use Mix.Task

  def run(_) do
    {:ok, _} = Application.ensure_all_started(:ex_compile_graph)

    Mix.Tasks.Compile.Elixir.run([])

    app_code_path = Path.join([Mix.Project.app_path(), "ebin"])
    :code.add_path(String.to_charlist(app_code_path))

    Mix.Shell.IO.info("Visit web UI at http://localhost:4040")
    Process.sleep(:infinity)
  end
end
