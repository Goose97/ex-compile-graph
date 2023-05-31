defmodule ExCompileGraph.Dependency.Cause do
  @type t :: %__MODULE__{
          name: atom(),
          origin_file: ExCompileGraph.file_path(),
          lines_span: {non_neg_integer(), non_neg_integer()}
        }

  defstruct [
    :name,
    :origin_file,
    :lines_span
  ]
end
