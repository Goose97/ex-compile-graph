import React, { useEffect, useRef, useState } from "react";
import { Spinner } from "@blueprintjs/core";
import * as d3 from "d3";

import GraphLegend from "./GraphLegend";
import ExplainDialog from "./ExplainDialog";
import { recompileDenpendencies } from "../index";
import type { Graph, VertexId, DependencyType } from "../index";
import type { DialogPage } from "./ExplainDialog";

interface IProps {
  loading?: boolean;
  graph?: Graph;
  focusedVertex?: VertexId;
  onSelectVertex?: (vertex: VertexId) => void;
  dependencyTypeFilter: Record<DependencyType, boolean>;
  onDependencyFiltersChange?: (value: boolean, type: DependencyType) => void;
}

type LinkType = "oneWay" | "twoWaySameType" | "twoWayDifferentType";

export const COLOR_BY_DEPENDENCY: Record<DependencyType, string> = {
  runtime: "#999999",
  exports: "#FFCD00",
  compile: "#DB005B",
};

const LINK_STROKE_WIDTH = 4;
const LINK_DISTANCE = 80;
const NODE_RADIUS_BASE = 5;
const NODE_RADIUS_GROWTH = 2;
const NODE_STROKE_WIDTH = 2;

const FOCUS_RING_COLOR = "#30A2FF";
const FOCUS_RING_DELTA = 6;
const FOCUS_RING_WIDTH = 3;

type D3Node = {
  id: VertexId;
  name: VertexId;
  recompileEdgeDegree: number;
};

type AugmentedD3Node = D3Node & {
  x: number;
  y: number;
};

type D3Edge = {
  source: VertexId;
  target: VertexId;
  dependencyType: DependencyType;
};

type AugmentedD3Edge = D3Edge & {
  source: D3Node;
  target: D3Node;
};

type RenderGraphParams = {
  vertices: D3Node[];
  edges: D3Edge[];
  width: number;
  height: number;
  nodeRadius?: (n: D3Node) => number;
  linkStyle?: (e: AugmentedD3Edge) => "arc" | "line";
  onClickNode?: (event: React.MouseEvent, node: D3Node) => void;
};
function renderGraph(params: RenderGraphParams) {
  const { vertices, edges, width, height } = params;
  const d3Nodes = vertices.map((v) => ({ ...v, index: undefined }));
  const d3Links = edges.map((e) => ({ ...e }));

  const forceNode = d3.forceManyBody();
  const forceLink = d3
    .forceLink(d3Links)
    .id((d) => d.id)
    .distance(LINK_DISTANCE);

  let simulation = d3
    .forceSimulation(d3Nodes)
    .force("link", forceLink)
    .force("charge", forceNode)
    .force("center", d3.forceCenter())
    .on("tick", ticked);

  const svg = d3
    .create("svg")
    .attr("width", width)
    .attr("height", height)
    .attr("viewBox", [-width / 2, -height / 2, width, height])
    .attr("style", "max-width: 100%; height: auto; height: intrinsic;");

  const arrowSize = 3;
  svg
    .append("defs")
    .selectAll("marker")
    .data<DependencyType>(["compile", "exports", "runtime"])
    .join("marker")
    .attr("id", (d) => `arrow-${d}`)
    .attr("viewBox", `0 0 ${arrowSize} ${arrowSize}`)
    .attr("refX", arrowSize * 1.4)
    .attr("refY", arrowSize / 2)
    .attr("markerWidth", arrowSize)
    .attr("markerHeight", arrowSize)
    .attr("orient", "auto-start-reverse")
    .append("path")
    .attr("fill", (value) => COLOR_BY_DEPENDENCY[value])
    .attr("d", `M0,0 L${arrowSize},${arrowSize / 2} L0,${arrowSize} z`);

  let links = svg
    .append("g")
    .attr("id", "links-container")
    .attr("fill", "none")
    .attr("stroke-width", LINK_STROKE_WIDTH)
    .attr("stroke-linecap", "round")
    .selectAll("line")
    .data(d3Links)
    .join("path");

  function applyLinkStyle(links: any) {
    links
      .attr("stroke", (d: AugmentedD3Edge) => {
        return COLOR_BY_DEPENDENCY[d.dependencyType];
      })
      .attr("marker-end", (d: AugmentedD3Edge) => {
        const url = new URL(`#arrow-${d.dependencyType}`, window.location.href);
        return `url(${url})`;
      })
      .attr("data-source", (d: AugmentedD3Edge) => d.source.id)
      .attr("data-target", (d: AugmentedD3Edge) => d.target.id);
  }

  applyLinkStyle(links);

  const nodeContainer = svg
    .append("g")
    .attr("id", "nodes-container")
    .attr("stroke", "#fff")
    .attr("stroke-width", NODE_STROKE_WIDTH);

  let nodes = nodeContainer.selectAll("circle").data(d3Nodes).join("circle");

  function applyNodeStyle(nodes: any) {
    nodes
      .attr("class", "graph-view-vertex")
      .attr("fill", "currentColor")
      .attr("id", (d) => d.id)
      .attr(
        "r",
        (d) => NODE_RADIUS_BASE + d.recompileEdgeDegree * NODE_RADIUS_GROWTH
      )
      .attr("fill", "currentColor")
      .call(drag(simulation))
      .on("click", (event, d) => params.onClickNode?.(event, d))
      .on("mouseover", (event, d) => {
        const tooltipEl = document.getElementsByClassName("graph-tooltip")[0];

        if (tooltipEl) {
          const [title, subtitle] = Array.from(tooltipEl.children);
          assertElementType(tooltipEl, tooltipEl instanceof HTMLDivElement);
          assertElementType(title, title instanceof HTMLSpanElement);
          assertElementType(subtitle, subtitle instanceof HTMLSpanElement);

          tooltipEl.style.left = `${event.clientX - 70}px`;
          tooltipEl.style.top = `${event.clientY - 80}px`;
          tooltipEl.style.display = "flex";
          title.innerText = d.id;
          subtitle.innerText = `Recompile degree: ${d.recompileEdgeDegree}`;
        }
      })
      .on("mouseout", () => {
        const tooltipEl = document.getElementsByClassName("graph-tooltip")[0];

        if (tooltipEl) {
          assertElementType(tooltipEl, tooltipEl instanceof HTMLDivElement);
          tooltipEl.style.display = "none";
        }
      });
  }

  applyNodeStyle(nodes);

  const focusRing = svg
    .append("circle")
    .attr("id", "focus-ring")
    .attr("display", "none")
    .attr("stroke-width", FOCUS_RING_WIDTH)
    .attr("stroke", FOCUS_RING_COLOR)
    .attr("fill", "none");

  function updateEdges(edges: D3Edge[]) {
    const d3NewLinks = edges.map((e) => ({ ...e }));

    const forceLink = d3
      .forceLink(d3NewLinks)
      .id((d) => d.id)
      .distance(LINK_DISTANCE);
    simulation = simulation.force("link", forceLink);

    links = links.data(d3NewLinks, function (d: AugmentedD3Edge) {
      return d
        ? `${d.source.id}|${d.target.id}`
        : `${this.dataset.source}|${this.dataset.target}`;
    });

    applyLinkStyle(links.enter().append("path"));
    links.exit().remove();

    links = d3.select("#links-container").selectAll("path");
    ticked();
  }

  function updateNodes(vertices: D3Node[]) {
    const d3NewNodes = vertices.map((n) => ({ ...n }));
    simulation = simulation.nodes(d3NewNodes);

    nodes = nodes.data(d3NewNodes, function (d: AugmentedD3Node) {
      return d ? d.id : this.id;
    });

    applyNodeStyle(nodes.enter().append("circle"));
    nodes.exit().remove();

    nodes = d3.select("#nodes-container").selectAll("circle");
    simulation.alpha(1);
    simulation.restart();
  }

  let focusVertex: VertexId | undefined;

  function setFocusVertex(vertex: VertexId | undefined) {
    focusVertex = vertex;
    if (focusVertex) ticked();
    else focusRing.attr("display", "none");
  }

  function linkArc(d: any) {
    const dx = d.target.x - d.source.x;
    const dy = d.target.y - d.source.y;
    const dr = Math.sqrt(dx * dx + dy * dy);
    return `M ${d.source.x},${d.source.y} A ${dr},${dr} 0 0,1 ${d.target.x},${d.target.y}`;
  }

  function linkLine(d: any) {
    return `M ${d.source.x},${d.source.y} L ${d.target.x},${d.target.y}`;
  }

  function ticked() {
    // Update links position
    links.attr("d", (d) => {
      const linkStyle = params.linkStyle
        ? params.linkStyle(d as AugmentedD3Edge)
        : "line";
      switch (linkStyle) {
        case "line":
          return linkLine(d);
        case "arc":
          return linkArc(d);
      }
    });

    // Update nodes position
    nodes.attr("cx", (d: any) => d.x).attr("cy", (d: any) => d.y);

    // Update focus ring position
    if (focusVertex !== undefined) {
      nodes
        .filter((n) => n.id === focusVertex)
        .each((n) => {
          focusRing
            .attr("cx", n.x)
            .attr("cy", n.y)
            .attr("r", nodeRadius(n) + FOCUS_RING_DELTA)
            .attr("display", "initial");
        });
    }
  }

  function drag(simulation: any) {
    function dragstarted(event: any) {
      if (!event.active) simulation.alphaTarget(0.3).restart();
      event.subject.fx = event.subject.x;
      event.subject.fy = event.subject.y;
    }

    function dragged(event: any) {
      event.subject.fx = event.x;
      event.subject.fy = event.y;
    }

    function dragended(event: any) {
      if (!event.active) simulation.alphaTarget(0);
      event.subject.fx = null;
      event.subject.fy = null;
    }

    return d3
      .drag()
      .on("start", dragstarted)
      .on("drag", dragged)
      .on("end", dragended);
  }

  return {
    svg: svg.node(),
    updateEdges,
    updateNodes,
    setFocusVertex,
  };
}

function nodeRadius(d) {
  const base = 5;
  return base + d.recompileEdgeDegree * 2;
}

const GraphView = (props: IProps) => {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogPage, setDialogPage] = useState<DialogPage | undefined>();

  const container = useRef<HTMLDivElement>(null);

  const graphComponents = useRef<any>();
  const computedEdges = useRef<D3Edge[]>();

  useEffect(() => {
    if (props.graph && container.current) {
      const vertices = props.graph.map((vertex) => ({
        id: vertex.id,
        name: vertex.id,
        recompileEdgeDegree: recompileDenpendencies(vertex).length,
      }));

      const edges = [];
      for (const vertex of props.graph) {
        // If a vertex has multiple edges to the same vertex, pick the one with highest precedent
        vertex.edges.sort((e1, e2) => {
          if (e1.to === e2.to) {
            const precedence = {
              compile: 0,
              exports: 1,
              runtime: 2,
            };

            return (
              precedence[e1.dependency_type] - precedence[e2.dependency_type]
            );
          } else {
            return e1.to > e2.to ? 1 : -1;
          }
        });

        const seenVertices = new Set();
        for (const edge of vertex.edges) {
          if (!seenVertices.has(edge.to)) {
            edges.push({
              source: edge.from,
              target: edge.to,
              dependencyType: edge.dependency_type,
            });

            seenVertices.add(edge.to);
          }
        }
      }

      const linkTypeTable = categorizedLinks(props.graph);
      computedEdges.current = edges;

      const { width, height } = container.current.getBoundingClientRect();

      if (graphComponents.current) {
        graphComponents.current.updateNodes(vertices);
        graphComponents.current.updateEdges(edges);
      } else {
        // First time render
        const graph = renderGraph({
          vertices,
          edges,
          width,
          height,
          linkStyle: (d) => {
            const linkType = getTableValue(
              linkTypeTable,
              d.source.id,
              d.target.id
            );

            switch (linkType) {
              case "oneWay":
              case "twoWaySameType":
                return "line";
              case "twoWayDifferentType":
                return "arc";
              default:
                return "line";
            }
          },
        });

        if (graph?.svg) container.current?.appendChild(graph.svg);
        graphComponents.current = graph;
      }
    }
  }, [props.graph, container.current]);

  useEffect(() => {
    if (graphComponents.current && computedEdges.current) {
      const edges = computedEdges.current.filter(
        (i) => props.dependencyTypeFilter[i.dependencyType]
      );
      graphComponents.current.updateEdges(edges);
    }
  }, [props.dependencyTypeFilter]);

  useEffect(() => {
    graphComponents.current?.setFocusVertex(props.focusedVertex);
  }, [props.focusedVertex]);

  return (
    <div className="graph-view" ref={container}>
      {props.loading && <Spinner className="loading-mask" />}
      <div className="graph-tooltip">
        <span className="graph-tooltip-title"></span>
        <span className="graph-tooltip-subtitle"></span>
      </div>
      <GraphLegend
        onExplainRequest={(page) => {
          setDialogPage(page);
          setDialogOpen(true);
        }}
        dependencyFilter={props.dependencyTypeFilter}
        onDependencyFilterToggle={props.onDependencyFiltersChange}
      />
      <ExplainDialog
        open={dialogOpen}
        page={dialogPage}
        onPageChange={setDialogPage}
        onClose={() => {
          setDialogOpen(false);
          setDialogPage(undefined);
        }}
      />
    </div>
  );
};

type Table<K extends string, V> = Record<K, Record<K, V>>;

function getTableValue<V>(
  table: Table<VertexId, V>,
  from: VertexId,
  to: VertexId
): V | null {
  const values = table[from];
  return values ? values[to] : null;
}

function setTableValue<V>(
  table: Table<VertexId, V>,
  from: VertexId,
  to: VertexId,
  value: V
) {
  const map = table[from];
  if (map) map[to] = value;
  else {
    table[from] = { [to]: value };
  }
}

function categorizedLinks(graph: Graph): Table<VertexId, LinkType> {
  const dependencyTypeTable: Table<VertexId, DependencyType> = {};
  const linkTypeTable: Table<VertexId, LinkType> = {};

  for (const vertex of graph) {
    for (const edge of vertex.edges) {
      setTableValue(
        dependencyTypeTable,
        edge.from,
        edge.to,
        edge.dependency_type
      );

      const reverseLinkDependencyType = getTableValue(
        dependencyTypeTable,
        edge.to,
        edge.from
      );
      if (
        reverseLinkDependencyType &&
        reverseLinkDependencyType === edge.dependency_type
      ) {
        // Update both current and reverse link
        setTableValue(linkTypeTable, edge.from, edge.to, "twoWaySameType");
        setTableValue(linkTypeTable, edge.to, edge.from, "twoWaySameType");
      } else if (reverseLinkDependencyType) {
        // Update both current and reverse link
        setTableValue(linkTypeTable, edge.from, edge.to, "twoWayDifferentType");
        setTableValue(linkTypeTable, edge.to, edge.from, "twoWayDifferentType");
      } else {
        // No reverse link
        setTableValue(linkTypeTable, edge.from, edge.to, "oneWay");
      }
    }
  }

  return linkTypeTable;
}

function assertElementType(
  element: Element,
  condition: unknown
): asserts condition {
  if (condition === false)
    throw new Error(`Unexpected element type ${typeof element}`);
}

export default GraphView;
