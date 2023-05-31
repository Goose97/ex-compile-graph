import React, { useState } from "react";
import { Icon, Switch } from "@blueprintjs/core";

import { COLOR_BY_DEPENDENCY } from "./index";
import type { DependencyType } from "../index";
import type { DialogPage } from "./ExplainDialog";

interface IProps {
  onExplainRequest?: (page: DialogPage) => void;
  dependencyFilter: Record<DependencyType, boolean>;
  onDependencyFilterToggle?: (value: boolean, type: DependencyType) => void;
}

const GraphLegend = (props: IProps) => {
  const [isExpand, setExpand] = useState(true);

  const expand = (
    <div className="graph-legend graph-legend-expand">
      <Icon
        icon="minimize"
        className="graph-legend-minimize"
        onClick={() => setExpand(false)}
      />
      <svg width="0" height="0">
        <defs>
          {Object.entries(COLOR_BY_DEPENDENCY).map(([type, color]) => (
            <marker
              key={type}
              id={`graph-legend__arrow-${type}`}
              viewBox="0 0 3 3"
              refX="0"
              refY="1.5"
              markerWidth="3"
              markerHeight="3"
              orient="auto-start-reverse"
            >
              <path fill={color} d="M 0,0 L 3,1.5 L 0,3 z" />
            </marker>
          ))}
        </defs>
      </svg>
      <p>
        <strong>A dot</strong> represents a file.&nbsp;
        <strong>The recompile degree</strong> of a dot is the amount of files
        which will recompile if this file recompiles. The bigger the degree, the
        bigger the dot
      </p>
      <div
        style={{
          marginBottom: "1rem",
          display: "flex",
          justifyContent: "center",
        }}
      >
        <svg viewBox="20 0 300 24" width="300" height="24">
          <circle cx="35" cy="12" r="7"></circle>
          <text x="50" y="16">
            Degree of 1
          </text>
          <circle cx="200" cy="12" r="11"></circle>
          <text x="220" y="16">
            Degree of 3
          </text>
        </svg>
      </div>

      <p>
        <strong>An arrow</strong>
        {` denotes a dependency between two files. An arrow pointing from
          file A to file B states: file A has a dependency on file B. The type of
          dependency is determine by the arrow's color`}
      </p>
      {Object.entries(COLOR_BY_DEPENDENCY).map(([type, color]) => {
        let description;
        switch (type) {
          case "compile":
            description = "Compile dependency";
            break;
          case "exports":
            description = "Exports dependency";
            break;
          case "runtime":
            description = "Runtime dependency";
            break;
        }

        return (
          <span key={type}>
            <svg viewBox="0 0 80 20" width="100px" height="20px">
              <path
                d="M 0,15 h 65"
                strokeWidth={4}
                strokeOpacity={1}
                stroke={color}
                markerEnd={`url(#graph-legend__arrow-${type})`}
              />
            </svg>

            <span
              className="graph-legend-dependency-type"
              style={{ borderBottom: "2px dotted", cursor: "pointer" }}
              onClick={() => {
                let explainPage: DialogPage;
                switch (type) {
                  case "compile":
                    explainPage = "compileDependency";
                    break;
                  case "exports":
                    explainPage = "exportsDependency";
                    break;
                  case "runtime":
                    explainPage = "runtimeDependency";
                    break;
                  default:
                    throw new Error(`Unexpected dependency type ${type}`);
                }

                props.onExplainRequest?.(explainPage);
              }}
            >
              {description}
            </span>

            <Switch
              className="graph-legend-switch"
              checked={props.dependencyFilter[type as DependencyType]}
              onChange={(e) => {
                const value = e.currentTarget.checked;
                props.onDependencyFilterToggle?.(value, type as DependencyType);
              }}
            />
          </span>
        );
      })}
    </div>
  );

  const collapse = (
    <div
      className="graph-legend graph-legend-collapse"
      onClick={() => setExpand(true)}
    >
      <Icon icon="info-sign" />
    </div>
  );

  return isExpand ? expand : collapse;
};

export default GraphLegend;
