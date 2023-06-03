defmodule ExCompileGraphWeb.Server.SocketHandler do
  @behaviour :cowboy_websocket
  require Logger

  def init(request, state) do
    {:cowboy_websocket, request, state}
  end

  def websocket_init(state), do: {:ok, state}

  def websocket_handle({:text, req_payload}, state) do
    req_payload = Jason.decode!(req_payload)

    %{
      "sequence" => sequence,
      "request" => %{"type" => req_type}
    } = req_payload

    res_payload =
      case req_type do
        "getGraph" ->
          ExCompileGraph.get_graph()

        "getDependencyExplanation" ->
          %{"source" => source, "sink" => sink, "reason" => reason} =
            req_payload["request"]["payload"]

          ExCompileGraph.get_recompile_dependency_causes(source, sink, String.to_atom(reason))
      end

    serialized = Jason.encode!(%{sequence: sequence, payload: res_payload})
    {:reply, {:text, serialized}, state}
  rescue
    error ->
      Logger.error("""
      #{__MODULE__}: error while handling socket requests
      - error: #{inspect(error, pretty: true)}
      - request: #{inspect(error, pretty: true)}
      """)

      {:reply, {:text, ""}, state}
  end

  def websocket_info(_info, state) do
    {:ok, state}
  end
end
