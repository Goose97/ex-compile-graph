import React, { useContext, useState } from "react";
import { Icon, Collapse, Spinner } from "@blueprintjs/core";

import DependencyExplanation from "./DependencyExplanation";
import { ApiContext, RecompileDepedencyReason } from "../index";
import type {
  VertexId,
  RecompileDenpendency,
  RecompileDepedencyExplanation,
} from "../index";

interface IProps {
  dependencies: RecompileDenpendency[];
  sinkVertex: VertexId;
  // Empty vertex means we are un-hovering from an item
  onHover?: (vertex: VertexId) => void;
  onUnhover?: (vertex: VertexId) => void;
}

const RecompileDependenciesList = (props: IProps) => {
  const api = useContext(ApiContext);
  const [loading, setLoading] = useState(false);
  const [activeItem, setActiveItem] = useState<VertexId>();
  const [detailedExplanation, setDetailedExplanation] = useState<
    RecompileDepedencyExplanation[]
  >([]);

  const fetchExplanation = (payload: {
    source: VertexId;
    sink: VertexId;
    reason: RecompileDepedencyReason;
  }) => {
    setLoading(true);
    return api
      .request({ type: "getDependencyExplanation", payload })
      .then(setDetailedExplanation)
      .finally(() => setLoading(false));
  };

  return (
    <ul className="recompiles-dependencies-list">
      {props.dependencies.map((item) => (
        <li
          key={item.id}
          className="recompiles-dependency-item recompiles-dependency-details"
          onClick={() => {
            setDetailedExplanation([]);

            if (item.id === activeItem) {
              setActiveItem(undefined);
            } else {
              const payload = {
                source: item.id,
                sink: props.sinkVertex,
                reason: item.reason,
              };

              setActiveItem(item.id);
              fetchExplanation(payload);
            }
          }}
        >
          <span>
            <Icon icon="chevron-right" /> {item.id}
          </span>

          <Collapse
            isOpen={item.id === activeItem}
            keepChildrenMounted={true}
            className={`recompiles-dependency-item-collapsible ${
              item.id === activeItem ? "active" : ""
            }`}
          >
            {loading && <Spinner className="loading-mask" />}
            {detailedExplanation.map((i) => (
              <DependencyExplanation value={i} key={i.source} />
            ))}
            <span className="file-name-box">{props.sinkVertex}</span>
          </Collapse>
        </li>
      ))}
    </ul>
  );
};

export default RecompileDependenciesList;
