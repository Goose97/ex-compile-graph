defmodule ExCompileGraphWeb.Server do
  use Plug.Router

  plug(
    Plug.Static,
    at: "/",
    from: {:ex_compile_graph, "priv/static"},
    only: ~w(favicon.ico index.html index.js index.css prism.js prism.css)
  )

  plug(:match)
  plug(:dispatch)

  get "/" do
    path =
      Path.join([
        :code.priv_dir(:ex_compile_graph) |> to_string(),
        "static",
        "index.html"
      ])

    send_file(conn, 200, path)
  end

  get "/ws" do
    Plug.Conn.upgrade_adapter(conn, :websocket, {__MODULE__.SocketHandler, nil, %{}})
  end
end
