import React, { useEffect, useState } from "react";
import { Collapse, Icon } from "@blueprintjs/core";

import type { CodeSnippet } from "../index";

interface IProps {
  id: string;
  value: CodeSnippet;
  header?: string;
  collapsible?: boolean;
  onClick?: (event: React.MouseEvent) => void;
}

const MAX_ATTEMPT = 25;
function tryHighlight(key: string, attempt = 0) {
  if (attempt > MAX_ATTEMPT) return;
  const codeBlock = document.getElementById(`code-snippet-${key}`);
  if (codeBlock) {
    if (codeBlock.getAttribute("highlighted") === key) return;

    Prism.highlightAll();
    codeBlock.setAttribute("highlighted", key);
  } else {
    setTimeout(() => tryHighlight(key, attempt + 1), 25);
  }
}

const CodeSnippet = (props: IProps) => {
  const [open, setOpen] = useState(true);
  const code = (
    <pre
      data-start={props.value.lines_span[0]}
      data-line={`${props.value.highlight[0]}-${props.value.highlight[1]}`}
      className="line-numbers"
      style={{ cursor: "text" }}
    >
      <code className="language-elixir">{props.value.content}</code>
    </pre>
  );
  const icon = open ? <Icon icon="collapse-all" /> : <Icon icon="expand-all" />;

  useEffect(() => {
    tryHighlight(props.id);
  }, []);

  useEffect(() => {
    if (open) tryHighlight(props.id);
  }, [open]);

  return (
    <div
      className="code-snippet line-numbers"
      id={`code-snippet-${props.id}`}
      onClick={props.onClick}
    >
      {props.header && (
        <span
          className="code-snippet-header"
          onClick={() => setOpen((s) => !s)}
        >
          {props.collapsible && icon}
          {props.header}
        </span>
      )}

      {props.collapsible ? (
        <Collapse keepChildrenMounted isOpen={open}>
          {code}
        </Collapse>
      ) : (
        code
      )}
    </div>
  );
};

export default CodeSnippet;
