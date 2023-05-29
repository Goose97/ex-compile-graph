defmodule ExCompileGraph.Server do
  def child_spec(_opts) do
    %{
      id: __MODULE__,
      start: {__MODULE__, :start_link, []}
    }
  end

  def start_link() do
    pid = spawn_link(&server_loop/0)
    {:ok, pid}
  end

  def server_loop() do
    line = IO.binread(:stdio, :line)

    if line != :eof do
      case Regex.run(~r/^C\[(\d+)\]:(.+)?\n$/, line) do
        [_, request_id, payload] ->
          request =
            case Jason.decode!(payload) do
              %{"type" => "init"} ->
                :init

              %{"type" => "get_files"} ->
                :get_files

              %{"type" => "get_dependency_causes"} = params ->
                {:get_dependency_causes, Map.take(params, ["source", "sink", "reason"])}
            end

          response = dispatch(request)
          IO.puts("S[#{request_id}]:#{Jason.encode!(response)}\n")

        _ ->
          IO.puts(
            :stderr,
            "Ignore invalid client requests. Expect requests format C[<request_id>]:<payload>, instead got #{line}"
          )
      end

      server_loop()
    else
      System.stop(0)
    end
  end

  def dispatch(:init) do
    ExCompileGraph.init()

    :ok
  end

  def dispatch(:get_files) do
    for %{id: vertex_id, recompile_dependencies: recompile_dependencies} <-
          ExCompileGraph.get_graph() do
      recompile_dependencies =
        Enum.map(recompile_dependencies, fn dependency ->
          Map.update!(dependency, :dependency_chain, fn chain ->
            Enum.map(chain, &Tuple.to_list/1)
          end)
        end)

      %{path: vertex_id, recompile_dependencies: recompile_dependencies}
    end
    # TODO: remove this
    |> Enum.filter(&String.contains?(&1.path, "fixtures"))
  end

  def dispatch({:get_dependency_causes, params}) do
    File.write!("ui/debug.log", "Requesting #{inspect(params)}\n")

    result =
      ExCompileGraph.get_recompile_dependency_causes(
        params["source"],
        params["sink"],
        String.to_existing_atom(params["reason"])
      )

    File.write!("ui/debug.log", "Response #{inspect(result)}\n")
    result
  end
end
