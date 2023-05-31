import React, { useEffect, useRef } from "react";
import { Spinner } from "@blueprintjs/core";
import * as d3 from "d3";

import type { Graph } from "./index";

interface IProps {
  loading?: boolean;
  data: Graph | null;
}

const COLOR_BY_DEPENDENCY = {
  runtime: "#999999",
  exports: "#FFE569",
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
  console.log(nodes, "nodes");
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
  const forceNode = d3.forceManyBody();
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
    .attr("stroke", typeof linkStroke !== "function" ? linkStroke : null)
    .attr("stroke-opacity", linkStrokeOpacity)
    .attr(
      "stroke-width",
      typeof linkStrokeWidth !== "function" ? linkStrokeWidth : null
    )
    .attr("stroke-linecap", linkStrokeLinecap)
    .selectAll("line")
    .data(links)
    .join("line")
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

  function ticked() {
    link
      .attr("x1", (d) => d.source.x)
      .attr("y1", (d) => d.source.y)
      .attr("x2", (d) => d.target.x)
      .attr("y2", (d) => d.target.y);

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

  return Object.assign(svg.node(), { scales: { color } });
}

const GraphView = (props: IProps) => {
  const container = useRef<HTMLDivElement>();

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

      const { width, height } = container.current.getBoundingClientRect();
      const chart = ForceGraph(
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

              console.log("im here in mouseover");
              tooltipEl.style.left = `${event.clientX - 70}px`;
              tooltipEl.style.top = `${event.clientY - 70}px`;
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
              console.log("im here in mouseout");
              tooltipEl.style.display = "none";
            }
          },
          nodeRadius: (d) => {
            const base = 5;
            return base + d.recompileEdgeDegree * 2;
          },
          linkStrokeWidth: 4,
          linkStrokeOpacity: 1,
          linkStroke: (d) => COLOR_BY_DEPENDENCY[d.dependencyType],
          linkDistance: 80,
          width,
          height,
        }
      );

      container.current?.appendChild(chart);

      console.log(props.data, "graph");
    }
  }, [props.data, container.current]);

  return (
    <div className="graph-view" ref={container}>
      {props.loading && <Spinner className="graph-view-loading" />}
      <div className="graph-tooltip">
        <span className="graph-tooltip-title"></span>
        <span className="graph-tooltip-subtitle"></span>
      </div>
    </div>
  );
};

function assertElementType(
  element: Element,
  condition: unknown
): asserts condition {
  if (condition === false)
    throw new Error(`Unexpected element type ${typeof element}`);
}

export default GraphView;
