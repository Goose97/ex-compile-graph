import React from "react";

import CodeSnippet from "./CodeSnippet";

import type { RecompileDepedencyExplanation } from "../index";
import { COLOR_BY_DEPENDENCY } from "../GraphView";

interface IProps {
  value: RecompileDepedencyExplanation;
}

const LINK_LENGTH = 120;

const DependencyExplanation = (props: IProps) => {
  let content: React.ReactNode;
  switch (props.value.type) {
    case "compile":
      content = "has a compile-time dependency on";
      break;
    case "exports":
      content = "has an exports dependency on";
      break;
    case "runtime":
      content = <span>has a runtime dependency on</span>;
      content =
        props.value.intermediates.length > 0 ? (
          <>
            {content}&nbsp;
            {`(transitively through ${props.value.intermediates.length} files)`}
          </>
        ) : (
          content
        );
      break;
  }

  const noSnippet: CodeSnippet = {
    content: "# Can not find the code causing\nthis dependency",
    lines_span: [1, 2],
    highlight: [1, 2],
  };

  return (
    <div className="explanation-item">
      {["exports", "compile"].includes(props.value.type) ? (
        <CodeSnippet
          id={props.value.source}
          value={
            props.value.snippets.length > 0
              ? props.value.snippets[0]
              : noSnippet
          }
          collapsible={true}
          header={props.value.source}
          onClick={(e) => e.stopPropagation()}
        />
      ) : (
        <span className="file-name-box">{props.value.source}</span>
      )}

      <div className="explanation-dependency-link">
        <svg
          viewBox={`0 0 20 ${LINK_LENGTH}`}
          width="20px"
          height={`${LINK_LENGTH}px`}
          stroke={COLOR_BY_DEPENDENCY[props.value.type]}
        >
          <path
            d={`M10,0 V${LINK_LENGTH - 10}`}
            strokeWidth="2"
            strokeDasharray="8"
          />
          <path
            d={`M5,${LINK_LENGTH - 12} l5,10 l5,-10`}
            strokeWidth="2"
            fill="none"
          />
        </svg>
        <span>{content}</span>
      </div>
    </div>
  );
};

export default DependencyExplanation;
