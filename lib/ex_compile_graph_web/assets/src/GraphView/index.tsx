import React, { useEffect, useRef, useState } from "react";
import { Spinner } from "@blueprintjs/core";
import * as d3 from "d3";

import GraphLegend from "./GraphLegend";
import ExplainDialog from "./ExplainDialog";
import type { Graph, VertexId, DependencyType } from "../index";
import type { DialogPage } from "./ExplainDialog";

interface IProps {
  loading?: boolean;
  data: Graph | null;
}

type LinkType = "oneWay" | "twoWaySameType" | "twoWayDifferentType";
type LinkStyle = "line" | "arc";

export const COLOR_BY_DEPENDENCY: Record<DependencyType, string> = {
  runtime: "#999999",
  exports: "#FFCD00",
  compile: "#DB005B",
};

// Copyright 2021 Observable, Inc.
// Released under the ISC license.
// https://observablehq.com/@d3/force-directed-graph
function ForceGraph(
  {
    nodes, // an iterable of node objects (typically [{id}, …])
    links, // an iterable of link objects (typically [{source, target}, …])
  },
  {
    nodeId = (d) => d.id, // given d in nodes, returns a unique identifier (string)
    nodeGroup, // given d in nodes, returns an (ordinal) value for color
    nodeGroups, // an array of ordinal values representing the node groups
    nodeTitle, // given d in nodes, a title string
    nodeFill = "currentColor", // node stroke fill (if not using a group color encoding)
    nodeStroke = "#fff", // node stroke color
    nodeStrokeWidth = 1.5, // node stroke width, in pixels
    nodeStrokeOpacity = 1, // node stroke opacity
    nodeRadius = 5, // given d in nodes, a radius in pixels
    nodeStrength,
    onMouseOverNode, // a callback trigger when move mouse over nodes
    onMouseOutNode, // a callback trigger when move mouse out of nodes
    linkSource = ({ source }) => source, // given d in links, returns a node identifier string
    linkTarget = ({ target }) => target, // given d in links, returns a node identifier string
    linkStroke = "#999", // link stroke color
    linkStrokeOpacity = 0.6, // link stroke opacity
    linkStrokeWidth = 1.5, // given d in links, returns a stroke width in pixels
    linkStrokeLinecap = "round", // link stroke linecap
    linkDistance = 50,
    linkStrength,
    linkStyle, // given d in links, return render style of links (LinkStyle)
    colors = d3.schemeTableau10, // an array of color strings, for the node groups
    width = 640, // outer width, in pixels
    height = 400, // outer height, in pixels
    invalidation, // when this promise resolves, stop the simulation
  } = {}
) {
  // Compute values.
  const N = d3.map(nodes, nodeId).map(intern);
  const LS = d3.map(links, linkSource).map(intern);
  const LT = d3.map(links, linkTarget).map(intern);
  if (nodeTitle === undefined) nodeTitle = (_, i) => N[i];
  const T = nodeTitle === null ? null : d3.map(nodes, nodeTitle);
  const G = nodeGroup === null ? null : d3.map(nodes, nodeGroup).map(intern);
  const W =
    typeof linkStrokeWidth !== "function"
      ? null
      : d3.map(links, linkStrokeWidth);
  const NR =
    typeof nodeRadius !== "function" ? null : d3.map(nodes, nodeRadius);
  const L = typeof linkStroke !== "function" ? null : d3.map(links, linkStroke);
  const LA = links.map(
    (d) => `url(${new URL(`#arrow-${d.dependencyType}`, location)})`
  );

  // Replace the input nodes and links with mutable objects for the simulation.
  nodes = d3.map(nodes, (n) => ({ ...n }));
  links = d3.map(links, (l) => ({ ...l }));

  // Compute default domains.
  if (G && nodeGroups === undefined) nodeGroups = d3.sort(G);

  // Construct the scales.
  const color = nodeGroup === null ? null : d3.scaleOrdinal(nodeGroups, colors);

  // Construct the forces.
  const forceNode = d3.forceManyBody().strength(-150);
  const forceLink = d3
    .forceLink(links)
    .id(({ index: i }) => N[i])
    .distance(linkDistance);

  if (nodeStrength !== undefined) forceNode.strength(nodeStrength);
  if (linkStrength !== undefined) forceLink.strength(linkStrength);

  const simulation = d3
    .forceSimulation(nodes)
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

  let tooltip = null;
  let tooltipTitle = null;
  let tooltipSubtitle = null;

  const arrowSize = 3;
  svg
    .append("defs")
    .selectAll("marker")
    .data(["compile", "exports", "runtime"])
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

  const link = svg
    .append("g")
    .attr("fill", "none")
    .attr("stroke", typeof linkStroke !== "function" ? linkStroke : null)
    .attr("stroke-opacity", linkStrokeOpacity)
    .attr(
      "stroke-width",
      typeof linkStrokeWidth !== "function" ? linkStrokeWidth : null
    )
    .attr("stroke-linecap", linkStrokeLinecap)
    .selectAll("line")
    .data(links)
    .join("path")
    .attr("marker-end", (d) => {
      return LA[d.index];
    });

  const node = svg
    .append("g")
    .attr("fill", nodeFill)
    .attr("stroke", nodeStroke)
    .attr("stroke-opacity", nodeStrokeOpacity)
    .attr("stroke-width", nodeStrokeWidth)
    .selectAll("circle")
    .data(nodes)
    .join("circle")
    .call(drag(simulation))
    .on("mouseover", (event, d) => {
      if (onMouseOverNode) onMouseOverNode(event, d);
    })
    .on("mouseout", (event, d) => {
      if (onMouseOutNode) onMouseOutNode(event, d);
    });

  tooltip = svg.append("g").style("display", "none");
  const tooltipText = tooltip.append("text").attr("x", "0").attr("y", "0");
  tooltipTitle = tooltipText.append("tspan").attr("x", "0").attr("dy", "1.2em");
  tooltipSubtitle = tooltipText
    .append("tspan")
    .attr("x", "0")
    .attr("dy", "1.2em");

  if (W) link.attr("stroke-width", ({ index: i }) => W[i]);
  if (L) link.attr("stroke", ({ index: i }) => L[i]);
  if (G) node.attr("fill", ({ index: i }) => color(G[i]));
  if (T) node.append("title").text(({ index: i }) => T[i]);
  if (NR) node.attr("r", ({ index: i }) => NR[i]);
  if (invalidation) invalidation.then(() => simulation.stop());

  function intern(value) {
    return value !== null && typeof value === "object"
      ? value.valueOf()
      : value;
  }

  function linkArc(d) {
    const dx = d.target.x - d.source.x;
    const dy = d.target.y - d.source.y;
    const dr = Math.sqrt(dx * dx + dy * dy);
    return `M ${d.source.x},${d.source.y} A ${dr},${dr} 0 0,1 ${d.target.x},${d.target.y}`;
  }

  function linkLine(d) {
    return `M ${d.source.x},${d.source.y} L ${d.target.x},${d.target.y}`;
  }

  function ticked() {
    link.attr("d", (d) => {
      const style: LinkStyle = linkStyle ? linkStyle(d) : "line";
      switch (style) {
        case "line":
          return linkLine(d);
        case "arc":
          return linkArc(d);
      }
    });
    node.attr("cx", (d) => d.x).attr("cy", (d) => d.y);
  }

  function drag(simulation) {
    function dragstarted(event) {
      if (!event.active) simulation.alphaTarget(0.3).restart();
      event.subject.fx = event.subject.x;
      event.subject.fy = event.subject.y;
    }

    function dragged(event) {
      event.subject.fx = event.x;
      event.subject.fy = event.y;
    }

    function dragended(event) {
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
    svg: Object.assign(svg.node(), { scales: { color } }),
    link,
    node,
  };
}

function updateLinkFilter(
  link: any,
  dependencyTypeFilter: Record<DependencyType, boolean>
) {
  link.attr("opacity", (d) => {
    return dependencyTypeFilter[d.dependencyType as DependencyType] ? 1 : 0;
  });
}

const GraphView = (props: IProps) => {
  const [dependencyTypeFilter, setDependencyTypeFiler] = useState<
    Record<DependencyType, boolean>
  >({ compile: true, exports: true, runtime: true });

  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogPage, setDialogPage] = useState<DialogPage | undefined>();

  const container = useRef<HTMLDivElement>(null);

  const graphComponents = useRef<any>();

  useEffect(() => {
    if (props.data && container.current) {
      const vertices = props.data.map((vertex) => ({
        id: vertex.id,
        name: vertex.id,
        recompileEdgeDegree: vertex.recompile_dependencies.length,
      }));

      const edges = [];
      for (const vertex of props.data) {
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

      const linkTypeTable = categorizedLinks(props.data);

      const { width, height } = container.current.getBoundingClientRect();
      const graph = ForceGraph(
        { nodes: vertices, links: edges },
        {
          nodeId: (d) => d.id,
          nodeGroup: null,
          nodeTitle: null,
          onMouseOverNode: (event, d) => {
            const tooltipEl =
              document.getElementsByClassName("graph-tooltip")[0];

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
          },
          onMouseOutNode: () => {
            const tooltipEl =
              document.getElementsByClassName("graph-tooltip")[0];

            if (tooltipEl) {
              assertElementType(tooltipEl, tooltipEl instanceof HTMLDivElement);
              tooltipEl.style.display = "none";
            }
          },
          nodeRadius: (d) => {
            const base = 5;
            return base + d.recompileEdgeDegree * 2;
          },
          linkStyle: (d) => {
            const linkType = getTableValue(
              linkTypeTable,
              d.source.id,
              d.target.id
            );

            switch (linkType) {
              case "oneWay":
                return "line";
              case "twoWaySameType":
                return "line";
              case "twoWayDifferentType":
                return "arc";
              default:
                return "line";
            }
          },
          linkStrokeWidth: 4,
          linkStrokeOpacity: 1,
          linkStroke: (d) => COLOR_BY_DEPENDENCY[d.dependencyType],
          linkDistance: 80,
          width,
          height,
        }
      );

      container.current?.appendChild(graph.svg);
      graphComponents.current = graph;
    }
  }, [props.data, container.current]);

  useEffect(() => {
    if (graphComponents.current) {
      updateLinkFilter(graphComponents.current.link, dependencyTypeFilter);
    }
  }, [dependencyTypeFilter]);

  return (
    <div className="graph-view" ref={container}>
      {props.loading && <Spinner className="graph-view-loading" />}
      <div className="graph-tooltip">
        <span className="graph-tooltip-title"></span>
        <span className="graph-tooltip-subtitle"></span>
      </div>
      <GraphLegend
        onExplainRequest={(page) => {
          setDialogPage(page);
          setDialogOpen(true);
        }}
        dependencyFilter={dependencyTypeFilter}
        onDependencyFilterToggle={(value, type) => {
          if (graphComponents.current) {
            const cloned = { ...dependencyTypeFilter };
            cloned[type] = value;
            setDependencyTypeFiler(cloned);
          }
        }}
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

function getTableValue<V, T extends Map<VertexId, Record<VertexId, V>>>(
  table: T,
  from: VertexId,
  to: VertexId
): V | null {
  const values = table.get(from);
  return values ? values[to] : null;
}

function setTableValue<V, T extends Map<VertexId, Record<VertexId, V>>>(
  table: T,
  from: VertexId,
  to: VertexId,
  value: V
) {
  const map = table.get(from);
  if (map) map[to] = value;
  else {
    table.set(from, { [to]: value });
  }
}

function categorizedLinks(
  graph: Graph
): Map<VertexId, Record<VertexId, LinkType>> {
  const dependencyTypeTable = new Map<
    VertexId,
    Record<VertexId, DependencyType>
  >();
  const linkTypeTable = new Map<VertexId, Record<VertexId, LinkType>>();

  for (const vertex of graph) {
    for (const edge of vertex.edges) {
      setTableValue<DependencyType, typeof dependencyTypeTable>(
        dependencyTypeTable,
        edge.from,
        edge.to,
        edge.dependency_type
      );

      const reverseLinkDependencyType = getTableValue<
        DependencyType,
        typeof dependencyTypeTable
      >(dependencyTypeTable, edge.to, edge.from);
      if (
        reverseLinkDependencyType &&
        reverseLinkDependencyType === edge.dependency_type
      ) {
        // Update both current and reverse link
        setTableValue<LinkType, typeof linkTypeTable>(
          linkTypeTable,
          edge.from,
          edge.to,
          "twoWaySameType"
        );
        setTableValue<LinkType, typeof linkTypeTable>(
          linkTypeTable,
          edge.to,
          edge.from,
          "twoWaySameType"
        );
      } else if (reverseLinkDependencyType) {
        // Update both current and reverse link
        setTableValue<LinkType, typeof linkTypeTable>(
          linkTypeTable,
          edge.from,
          edge.to,
          "twoWayDifferentType"
        );
        setTableValue<LinkType, typeof linkTypeTable>(
          linkTypeTable,
          edge.to,
          edge.from,
          "twoWayDifferentType"
        );
      } else {
        // No reverse link
        setTableValue<LinkType, typeof linkTypeTable>(
          linkTypeTable,
          edge.from,
          edge.to,
          "oneWay"
        );
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
