import React, { useEffect } from "react";
import { Dialog, DialogBody, Icon, IconSize } from "@blueprintjs/core";

export type DialogPage =
  | "compileDependency"
  | "exportsDependency"
  | "runtimeDependency";

interface IProps {
  open: boolean;
  page?: DialogPage;
  onPageChange?: (page: DialogPage) => void;
  onOpen?: () => void;
  onClose?: () => void;
}

const compileDependencySnippet = `
# A has a compile-time dependency on C because A require C
defmodule A do
  require C
end

# B has a compile-time dependency on C because B
# invokes C functions outside functions
defmodule B do
  C.x()
end

defmodule C do
  def x(), do: 1
end
`;

const exportsDependencySnippet = `
# A has a exports dependency on B because A import and using B public
# functions. A will recompiles if B add/remove a public function
defmodule A do
  import B

  def y(), do: x()
end

# B has an export dependency on C, caused by using C struct. B will recompiles
# if C changes its struct defnition
defmodule B do
  def x(), do: %C{}
end

defmodule C do
  defstruct [:a, :b, :c]
end
`;

const runtimeDependencySnippet = `
# A has an runtime dependency on B
defmodule A do
  def x(), do: x()
end

defmodule B do
  def x(), do: 1
end
`;

const PAGES: { id: DialogPage; title: string; content: React.ReactElement }[] =
  [
    {
      id: "compileDependency",
      title: "Compile-time dependencies",
      content: (
        <>
          <p>
            Compile-time dependencies are typically caused by using macros or
            invoking functions of other modules in the module body (outside of
            functions). If module A has a compile-time dependency on module B,
            module A <strong>DEFINITELY</strong> has to recompile whenever
            module B recompiles.
          </p>

          <div>
            <pre>
              <code className="language-elixir">
                {compileDependencySnippet}
              </code>
            </pre>
          </div>
        </>
      ),
    },
    {
      id: "exportsDependency",
      title: "Exports dependencies",
      content: (
        <>
          <p>
            Exports dependencies are formed when a module depends on another
            module API, namely structs and public functions, in compile time.
            For instance, when you import a module or use a struct from another
            module. If module A has an exports dependencies on module B, module
            A <strong>MAY OR MAY NOT</strong> recompile when module B
            recompiles. It only recompiles when the target module API changes,
            e.g. changes struct definitions or add/remote public functions.
          </p>

          <div>
            <pre>
              <code className="language-elixir">
                {exportsDependencySnippet}
              </code>
            </pre>
          </div>
        </>
      ),
    },
    {
      id: "runtimeDependency",
      title: "Runtime dependencies",
      content: (
        <>
          <p>
            Runtime dependencies are formed when a module invoke another module
            functions inside a function. If module A has a runtime dependency on
            module B, module A <strong>WILL NOT</strong> recompiles whenever
            module B recompiles.
          </p>

          <div>
            <pre>
              <code className="language-elixir">
                {runtimeDependencySnippet}
              </code>
            </pre>
          </div>
        </>
      ),
    },
  ];

const MAX_ATTEMPT = 25;
function tryHighlight(page: DialogPage, attempt = 0) {
  if (attempt > MAX_ATTEMPT) return;
  const codeBlock = document.getElementById("explain-dialog-code");
  if (codeBlock) {
    if (codeBlock.getAttribute("highlighted") === page) return;

    Prism.highlightAll();
    codeBlock.setAttribute("highlighted", page);
  } else {
    setTimeout(() => tryHighlight(page, attempt + 1), 25);
  }
}

function nextPage(currentPage: DialogPage) {
  const current = PAGES.findIndex((p) => p.id === currentPage);
  if (current === -1)
    throw new Error(`Current explain page ${currentPage} not found`);
  return PAGES[(current + 1) % PAGES.length];
}

function prevPage(currentPage: DialogPage) {
  const current = PAGES.findIndex((p) => p.id === currentPage);
  if (current === -1)
    throw new Error(`Current explain page ${currentPage} not found`);
  return PAGES[(current + PAGES.length - 1) % PAGES.length];
}

const ExplainDialog = (props: IProps) => {
  useEffect(() => {
    if (props.page) tryHighlight(props.page);
  }, [props.open, props.page]);

  const pageIndex = PAGES.findIndex((i) => i.id === props.page);
  const page = PAGES[pageIndex];
  if (props.page && !page)
    throw new Error(`Unexpected explain content ${props.page}`);

  return (
    <Dialog
      isOpen={props.open}
      className="explain-dialog"
      onClose={props.onClose}
      onOpened={props.onOpen}
      title={page?.title}
    >
      {page && (
        <DialogBody className="explain-dialog-body">
          <div id="explain-dialog-code">{page.content}</div>
          {pageIndex < PAGES.length - 1 && (
            <Icon
              icon="chevron-right"
              className="explain-dialog-next"
              size={IconSize.LARGE}
              onClick={() => props.onPageChange?.(nextPage(page.id).id)}
            />
          )}
          {pageIndex > 0 && (
            <Icon
              icon="chevron-left"
              className="explain-dialog-prev"
              size={IconSize.LARGE}
              onClick={() => props.onPageChange?.(prevPage(page.id).id)}
            />
          )}
        </DialogBody>
      )}
    </Dialog>
  );
};

export default ExplainDialog;
