import React from "react";
import { Spinner, Icon } from "@blueprintjs/core";
import type { Graph, VertexId, DependencyType } from "../index";

interface IProps {
  data?: Graph;
  loading: boolean;
  selectedVertex?: VertexId;
  onSelectVertex?: (vertex: VertexId) => void;
  onHoverVertex?: (vertex: VertexId) => void;
  onUnhoverVertex?: (vertex: VertexId) => void;
}

const SidePanel = (props: IProps) => {
  const sortedByRecompileDegree = (graph: Graph) => {
    const clone = [...graph];
    clone.sort(
      (a, b) =>
        b.recompile_dependencies.length - a.recompile_dependencies.length
    );
    return clone;
  };

  return (
    <div className="side-panel">
      <div className="side-panel-header">
        <h3>Overview</h3>
        <p>Files sorted by descending recompiles degree</p>
      </div>

      {props.loading && <Spinner className="loading-mask" />}
      {props.data ? (
        <ul className="recompiles-dependencies-list">
          {sortedByRecompileDegree(props.data).map((vertex) => (
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
                {vertex.recompile_dependencies.length}
              </span>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
};

export default SidePanel;
