import { createRoot } from "react-dom/client";
import React, { useEffect, useRef, useState } from "react";

import SidePanel from "./SidePanel";
import GraphView from "./GraphView";
import "./index.css";

type ApiRequest =
  | { type: "getGraph" }
  | { type: "getDependency"; payload: string };

type ApiResponseMap = {
  getGraph: Graph;
  getDependency: unknown;
};

type ApiResponse = unknown;

export type Graph = Vertex[];
type RecompileDepedencyReason =
  | "compile"
  | "exports_then_compile"
  | "exports"
  | "compile_then_runtime";

type VertexId = string;
type Vertex = {
  id: VertexId;
  edges: Edge[];
  recompile_dependencies: { id: VertexId; reason: RecompileDepedencyReason }[];
};

type Edge = {
  from: VertexId;
  to: VertexId;
  dependency_type: "runtime" | "exports" | "compile";
};

class ApiBase {
  socket: WebSocket | null;
  // Should not worry about overflow
  private sequence: number;
  private pendingRequests: Map<
    number,
    [type: ApiRequest["type"], successCb: (response: ApiResponse) => void]
  >;

  constructor() {
    this.socket = null;
    this.pendingRequests = new Map();
    this.sequence = 0;
  }

  connect(cb: () => void) {
    const ws = new WebSocket("ws://localhost:4040/ws");
    ws.addEventListener("open", () => {
      this.socket = ws;
      cb();
    });

    ws.addEventListener("message", (event) => {
      const { sequence, payload } = JSON.parse(event.data);
      const entry = this.pendingRequests.get(sequence);
      if (entry) {
        const [type, successCb] = entry;
        switch (type) {
          case "getGraph": {
            successCb(payload as ApiResponseMap["getGraph"]);
            break;
          }

          case "getDependency": {
            successCb(payload as ApiResponseMap["getDependency"]);
            break;
          }
        }
        successCb(payload);
        this.pendingRequests.delete(sequence);
      } else {
        console.warn(
          "Can't find matching a request for the response with sequence number #{sequence}"
        );
      }
    });
  }

  disconnect() {
    if (!this.socket) {
      console.warn("Socket hasn't connected or its already closed");
      return;
    }

    this.socket.close();
  }

  request<T extends ApiRequest>(
    request: T
  ): Promise<ApiResponseMap[T["type"]]> {
    return new Promise((resolve, reject) => {
      if (!this.socket) {
        return reject("Socket hasn't connected or its already closed");
      }

      if (this.socket.readyState !== 1) {
        return reject("Socket is not ready");
      }

      const currentSequence = this.sequence++;
      this.socket.send(JSON.stringify({ request, sequence: currentSequence }));
      this.pendingRequests.set(currentSequence, [request.type, resolve]);
    });
  }
}

const App = () => {
  const [connected, setConnected] = useState(false);
  const { current: api } = useRef(new ApiBase());

  const [isLoading, setLoading] = useState(false);
  const [graph, setGraph] = useState<Graph | null>(null);

  useEffect(() => {
    api.connect(() => {
      setConnected(true);

      api.request({ type: "getGraph" }).then((graph) => {
        setLoading(false);
        setGraph(graph);
      });
    });

    return api.disconnect;
  }, []);

  return connected ? (
    <>
      <GraphView loading={isLoading} data={graph} />
      <SidePanel />
    </>
  ) : null;
};

// Create your app

// Inject your application into the an element with the id `app`.
// Make sure that such an element exists in the dom ;)
const container = document.getElementById("app") as HTMLElement;
const root = createRoot(container);
root.render(<App />);
