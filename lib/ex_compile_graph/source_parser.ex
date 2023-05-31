defmodule ExCompileGraph.SourceParser do
  @moduledoc """
  Find expressions in module by scanning source code AST
  """

  @doc """
  Given a source file and a module, returns all import expressions originate from that module
  """
  def import_exprs(source_file, module) do
    with {:ok, ast} <- defmodule_expr(source_file, module) do
      {:defmodule, _, [_, [do: module_body]]} = ast

      # import expressions can appear at module top-level or nested inside def expressions
      scan_body(module_body, fn
        {:import, _, _} = expr ->
          [expr]

        {:def, _, [_, [do: function_body]]} ->
          case function_body do
            {:__block__, _, exprs} ->
              for {:import, _, _} = expr <- exprs, do: expr

            {:import, _, _} = expr ->
              [expr]

            _ ->
              []
          end

        _ ->
          []
      end)
    else
      {:error, :enoent} ->
        raise RuntimeError,
          message: """
          #{__MODULE__}.import_exprs: source file not found
          - source_file: #{source_file}
          """
    end
  end

  def import_target({:import, _, [{:__aliases__, _, names} | _]}), do: Module.concat(names)

  def import_target(expr),
    do:
      raise(RuntimeError,
        message:
          "#{__MODULE__}.import_target: expect an import expression, instead got #{inspect(expr)}"
      )

  # Body could be either a block or a single expression
  # Body could be a module body or a function body
  defp scan_body(body, callback) do
    case body do
      {:__block__, _, exprs} ->
        Enum.flat_map(exprs, callback)

      expr ->
        callback.(expr)
    end
  end

  @spec defmodule_expr(ExCompileGraph.file_path(), atom()) :: {:ok, Macro.t()} | {:error, atom()}
  def defmodule_expr(source_file, module) do
    with {:ok, bin} <- File.read(source_file),
         {:ok, quoted} <- Code.string_to_quoted(bin) do
      defmodule_expr_from_quoted(quoted, module)
    end
  end

  defp defmodule_expr_from_quoted(quoted, module) do
    case quoted do
      # Single module files
      {:defmodule, _, args} = ast ->
        if check_module_name(ast, module) do
          {:ok, ast}
        else
          # If this module is our parent module, recurse
          [{:__aliases__, _, names}, [do: module_body]] = args
          splitted = Module.split(module) |> Enum.map(&String.to_atom/1)

          if names == Enum.take(splitted, length(names)) do
            remain = Enum.drop(splitted, length(names)) |> Module.concat()
            defmodule_expr_from_quoted(module_body, remain)
          else
            {:error, :not_found}
          end
        end

      {:__block__, _, exprs} ->
        result =
          Enum.find_value(exprs, fn
            {:defmodule, _, _} = ast ->
              case defmodule_expr_from_quoted(ast, module) do
                {:ok, ast} -> ast
                _error -> false
              end

            _ ->
              false
          end)

        if result, do: {:ok, result}, else: {:error, :not_found}
    end
  end

  defp check_module_name(ast, module_name) do
    {:defmodule, _context, [{:__aliases__, _, names}, _do_block]} = ast
    Module.concat(names) == module_name
  end

  def struct_expr(source_file, filter_structs) when is_binary(source_file) do
    {:ok, bin} = File.read(source_file)
    {:ok, quoted} = Code.string_to_quoted(bin)
    struct_expr(quoted, filter_structs)
  end

  @doc """
  Find all struct usages in a AST, supports selective filters
  """
  @spec struct_expr(Macro.t(), [atom()]) :: [{atom(), Macro.t()}]
  def struct_expr(ast, filter_structs) do
    {_, {struct_exprs, _}} =
      Macro.prewalk(ast, {[], %{}}, fn
        {:alias, _, args} = expr, {result, alias_table} ->
          {alias_from, alias_to} =
            case args do
              [{:__aliases__, _, from}] -> {from, List.last(from)}
              # Alias to must not be a nested name
              [{:__aliases__, _, from}, [as: {:__aliases__, _, [to]}]] -> {from, to}
            end

          alias_table = Map.put(alias_table, alias_to, alias_from)
          {expr, {result, alias_table}}

        {:%, _, [{:__aliases__, _, struct_names}, _content]} = expr, {result, alias_table} ->
          unalias_name =
            case alias_table[hd(struct_names)] do
              nil -> Module.concat(struct_names)
              aliases -> Module.concat(aliases ++ tl(struct_names))
            end

          result = if unalias_name in filter_structs, do: [expr | result], else: result
          {expr, {result, alias_table}}

        expr, acc ->
          {expr, acc}
      end)

    struct_exprs
  end

  @doc """
  Given a source file, returns all struct defnitions in that file
  """
  @spec struct_defs!(ExCompileGraph.file_path()) :: [Macro.t()]
  def struct_defs!(source_file) do
    {:ok, bin} = File.read(source_file)
    {:ok, quoted} = Code.string_to_quoted(bin)

    pre = fn
      {:defstruct, _, _} = expr, {current_module, result} ->
        new_struct_def = {Module.concat(current_module), expr}
        {expr, {current_module, [new_struct_def | result]}}

      # Going in module def
      {:defmodule, _, [{:__aliases__, _, names}, _]} = expr, {current_module, result} ->
        current_module = if current_module, do: current_module ++ names, else: names
        {expr, {current_module, result}}

      {:def, _, _}, acc ->
        {nil, acc}

      expr, acc ->
        {expr, acc}
    end

    post = fn
      # Going out of module def
      {:defmodule, _, [{:__aliases__, _, names}, _]} = expr, {current_module, result} ->
        current_module = Enum.slice(current_module, 0, length(current_module) - length(names))
        {expr, {current_module, result}}

      expr, acc ->
        {expr, acc}
    end

    {_, {_, struct_defs}} = Macro.traverse(quoted, {nil, []}, pre, post)
    struct_defs
  end

  @doc """
  Returns the lines span in the source file of a given expression
  """
  @spec expr_lines_span(Macro.t()) :: {non_neg_integer(), non_neg_integer()}
  def expr_lines_span({:import, context, _}) do
    start = context[:line]
    {start, start}
  end

  # Struct expression
  def expr_lines_span({:%, context, _}) do
    start = context[:line]
    {start, start}
  end
end
