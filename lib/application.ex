defmodule ExCompileGraph.Application do
  use Application

  def start(_type, _args) do
    children = [
      {Plug.Cowboy, scheme: :http, plug: ExCompileGraphWeb.Server, port: 4040}
    ]

    Supervisor.start_link(children, strategy: :one_for_one)
  end
end
