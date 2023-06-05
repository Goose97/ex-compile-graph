import { createRoot } from "react-dom/client";
import React, {
  useEffect,
  useRef,
  useState,
  createContext,
  useMemo,
} from "react";

import SidePanel from "./SidePanel";
import GraphView from "./GraphView";
import "./index.css";

type ApiRequest =
  | { type: "getGraph" }
  | {
      type: "getDependencyExplanation";
      payload: {
        source: VertexId;
        sink: VertexId;
        reason: RecompileDepedencyReason;
      };
    };

type ApiResponseMap = {
  getGraph: Graph;
  getDependencyExplanation: RecompileDepedencyExplanation[];
};

type ApiResponse = unknown;

export type Graph = Vertex[];
export type VertexId = string;
export type DependencyType = "runtime" | "exports" | "compile";

export type RecompileDepedencyReason =
  | "compile"
  | "exports_then_compile"
  | "exports"
  | "compile_then_runtime";
export type RecompileDenpendency = {
  id: VertexId;
  reason: RecompileDepedencyReason;
};

export type Vertex = {
  id: VertexId;
  edges: Edge[];
  recompile_dependencies: RecompileDenpendency[];
};

export type Edge = {
  from: VertexId;
  to: VertexId;
  dependency_type: DependencyType;
};

type LinesSpan = [from: number, to: number];

export type CodeSnippet = {
  content: string;
  highlight: LinesSpan;
  lines_span: LinesSpan;
};

export type RecompileDepedencyExplanation = {
  intermediates: VertexId[];
  source: VertexId;
  type: DependencyType;
  snippets: CodeSnippet[];
};

export class ApiBase {
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

          case "getDependencyExplanation": {
            successCb(payload as ApiResponseMap["getDependencyExplanation"]);
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

// Perform a DFS on graph but only follow edges satisfy the filter
function getConnectedVertices(
  graph: Graph,
  vertex: Vertex,
  dependencyTypeFilter: Record<DependencyType, boolean>
): Set<VertexId> {
  const dictionary = new Map<VertexId, Vertex>();
  for (const vertex of graph) {
    dictionary.set(vertex.id, vertex);
  }

  const visited = new Set();
  const dfs = (vertexId: VertexId): Set<VertexId> => {
    const result = new Set<VertexId>();
    if (visited.has(vertexId)) return result;

    visited.add(vertexId);
    result.add(vertexId);

    const vertex = dictionary.get(vertexId) as Vertex;
    for (const edge of vertex.edges) {
      if (dependencyTypeFilter[edge.dependency_type]) {
        dfs(edge.to).forEach((i) => result.add(i));
      }
    }

    return result;
  };

  return dfs(vertex.id);
}

export function recompileDenpendencies(vertex: Vertex) {
  return vertex.recompile_dependencies.filter((i) =>
    ["compile", "compile_then_runtme"].includes(i.reason)
  );
}

export const ApiContext = createContext(new ApiBase());

const App = () => {
  const [connected, setConnected] = useState(false);
  const { current: api } = useRef(new ApiBase());

  const [isLoading, setLoading] = useState(false);
  const [graph, setGraph] = useState<Graph>();

  const [focusedVertex, setFocusedVertex] = useState<VertexId>();
  const [selectedVertex, setSelectedVertex] = useState<Vertex>();
  const [dependencyTypeFilter, setDependencyTypeFilter] = useState<
    Record<DependencyType, boolean>
  >({ compile: true, exports: true, runtime: true });

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

  const renderGraph = useMemo(() => {
    if (graph && selectedVertex) {
      const filteredVertices = getConnectedVertices(
        graph,
        selectedVertex,
        dependencyTypeFilter
      );

      return graph.filter((v) => filteredVertices.has(v.id));
    } else {
      return graph;
    }
  }, [graph, selectedVertex]);

  return connected ? (
    <ApiContext.Provider value={api}>
      <GraphView
        loading={isLoading}
        graph={renderGraph}
        focusedVertex={focusedVertex}
        onSelectVertex={(vertexId) => {
          if (graph) {
            const vertex = graph.find((v) => v.id === vertexId);
            if (!vertex)
              throw new Error(
                `Unexpected vertex ${vertexId} is selected in GraphView`
              );

            setFocusedVertex(vertex.id);
            setSelectedVertex(vertex);
          }
        }}
        onDependencyFiltersChange={(value, type) => {
          const clone = { ...dependencyTypeFilter };
          clone[type] = value;
          setDependencyTypeFilter(clone);
        }}
        dependencyTypeFilter={dependencyTypeFilter}
      />
      <SidePanel
        loading={isLoading}
        graph={graph}
        selectedVertex={selectedVertex}
        onSelectVertex={(vertexId) => {
          if (!vertexId) {
            setFocusedVertex(undefined);
            return setSelectedVertex(undefined);
          }

          if (graph) {
            const vertex = graph.find((v) => v.id === vertexId);
            if (!vertex)
              throw new Error(
                `Unexpected vertex ${vertexId} is selected in SidePanel`
              );
            setFocusedVertex(vertex.id);
            setSelectedVertex(vertex);
          }
        }}
        onHoverVertex={(vertexId) => setFocusedVertex(vertexId)}
        onUnhoverVertex={() => setFocusedVertex(undefined)}
      />
    </ApiContext.Provider>
  ) : null;
};

// Inject your application into the an element with the id `app`.
// Make sure that such an element exists in the dom ;)
const container = document.getElementById("app") as HTMLElement;
const root = createRoot(container);
root.render(<App />);
