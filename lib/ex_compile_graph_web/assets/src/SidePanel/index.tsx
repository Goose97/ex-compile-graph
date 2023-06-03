import React, { useEffect } from "react";
import { Spinner, Icon } from "@blueprintjs/core";

import { recompileDenpendencies } from "../index";
import RecompileDependenciesList from "./RecompileDependenciesList";
import type { Graph, Vertex, VertexId } from "../index";

interface IProps {
  graph?: Graph;
  loading: boolean;
  selectedVertex?: Vertex;
  onSelectVertex?: (vertex?: VertexId) => void;
  onHoverVertex?: (vertex: VertexId) => void;
  onUnhoverVertex?: (vertex: VertexId) => void;
}

const SidePanel = (props: IProps) => {
  const sortedByRecompileDegree = (graph: Graph) => {
    const clone = [...graph];
    clone.sort(
      (a, b) =>
        recompileDenpendencies(b).length - recompileDenpendencies(a).length
    );
    return clone;
  };

  const overviewTab = props.graph ? (
    <ul className="recompiles-dependencies-list">
      {sortedByRecompileDegree(props.graph).map((vertex) => (
        <li
          className="recompiles-dependency-item"
          key={vertex.id}
          onClick={() => props.onSelectVertex?.(vertex.id)}
          onMouseEnter={() => props.onHoverVertex?.(vertex.id)}
          onMouseOut={() => props.onUnhoverVertex?.(vertex.id)}
        >
          <span className="flex-row" style={{ gap: "8px" }}>
            <Icon icon="arrow-right" size={12} />
            <span>{vertex.id}</span>
          </span>

          <span className="recompiles-dependency-badge">
            {recompileDenpendencies(vertex).length}
          </span>
        </li>
      ))}
    </ul>
  ) : null;

  const detailedTab = props.selectedVertex && (
    <RecompileDependenciesList
      dependencies={recompileDenpendencies(props.selectedVertex)}
      sinkVertex={props.selectedVertex.id}
      onHover={props.onHoverVertex}
      onUnhover={props.onUnhoverVertex}
    />
  );

  const isInOverviewTab = props.selectedVertex === undefined;
  const backToOverview = () => props.onSelectVertex?.();

  useEffect(() => {
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape") backToOverview();
    });
  }, []);

  return (
    <div className="side-panel">
      <div className="side-panel-header">
        {isInOverviewTab ? (
          <>
            <h3>Overview</h3>
            <p>Files sorted by descending recompiles degree</p>
          </>
        ) : (
          <>
            <h3
              className="flex-row"
              style={{ gap: "8px", cursor: "pointer" }}
              onClick={backToOverview}
            >
              <Icon icon="arrow-left" /> Back to overview
            </h3>
            <p>{`Current file: ${props.selectedVertex?.id}`}</p>
            <p>{`There will be ${
              props.selectedVertex
                ? recompileDenpendencies(props.selectedVertex).length
                : 0
            } files recompiled if this file recompiled`}</p>
          </>
        )}
      </div>

      {isInOverviewTab ? overviewTab : detailedTab}
      {props.loading && <Spinner className="loading-mask" />}
    </div>
  );
};

export default SidePanel;
