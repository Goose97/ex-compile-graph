defmodule ExCompileGraphWeb.Server do
  use Plug.Router

  plug(
    Plug.Static,
    at: "/public",
    from: {:ex_compile_graph, "priv/static"},
    only: ~w(favicon.ico index.html index.js index.css prism.js prism.css)
  )

  plug(:match)
  plug(:dispatch)

  get "/" do
    send_resp(conn, 200, "hello")
  end

  get "/favicon.ico" do
    send_file(conn, 200, "lib/ex_compile_graph_web/assets/src/favicon.ico")
  end

  get "/ws" do
    Plug.Conn.upgrade_adapter(conn, :websocket, {__MODULE__.SocketHandler, nil, %{}})
  end
end
