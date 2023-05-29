defmodule ExCompileGraph.SourceParser do
  @moduledoc """
  Find expressions in module by scanning source code AST
  """

  @doc """
  Given a source file and a module, returns all import expressions originate from that module
  """
  @spec scan_module_exprs(ExCompileGraph.file_path(), atom(), :import | :require) :: [Macro.t()]
  def scan_module_exprs(source_file, module, expr) do
    with {:ok, ast} <- defmodule_expr(source_file, module) do
      {:defmodule, _, [_, [do: module_body]]} = ast

      # import expressions can appear at module top-level or nested inside def expressions
      scan_body(module_body, fn
        {^expr, _, _} = expr ->
          [expr]

        {:def, _, [_, [do: function_body]]} ->
          case function_body do
            {:__block__, _, exprs} ->
              for {^expr, _, _} = expr <- exprs, do: expr

            {^expr, _, _} = expr ->
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
          #{__MODULE__}.scan_module_exprs: source file not found
          - source_file: #{source_file}
          """

      {:error, :not_found} ->
        raise RuntimeError,
          message: """
          #{__MODULE__}.scan_module_exprs: module is not found in source file
          - source_file: #{source_file}
          - module: #{module}
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

  def require_target({:require, _, [{:__aliases__, _, names}]}), do: Module.concat(names)

  def require_target(expr),
    do:
      raise(RuntimeError,
        message:
          "#{__MODULE__}.require_target: expect an require expression, instead got #{inspect(expr)}"
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

  @spec struct_expr(Macro.t(), [atom()]) :: [{atom(), Macro.t()}]
  @doc """
  Find all struct usages in a AST, supports selective filters
  """
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
          unalias_name = expand_alias(struct_names, alias_table)
          result = if unalias_name in filter_structs, do: [expr | result], else: result
          {expr, {result, alias_table}}

        expr, acc ->
          {expr, acc}
      end)

    struct_exprs
  end

  defp expand_alias(alias_name, alias_table) do
    case alias_table[hd(alias_name)] do
      # No alias
      nil -> Module.concat(alias_name)
      aliases -> Module.concat(aliases ++ tl(alias_name))
    end
  end

  @spec struct_defs!(ExCompileGraph.file_path()) :: [Macro.t()]
  @doc """
  Given a source file, return all struct defnitions in that file
  """
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

  @spec macro_exprs(ExCompileGraph.file_path(), atom()) :: [Macro.t()]
  @doc """
  Given a source file and a module contains macro definitions, return all macro expressions in
  the source file
  """
  def macro_exprs(source_file, macro_module) do
    {:ok, bin} = File.read(source_file)
    {:ok, quoted} = Code.string_to_quoted(bin)
    macros = macro_module.__info__(:macros)

    initial_state = %{
      alias_table: %{},
      is_import: false,
      is_require: false,
      current_module: [],
      result: []
    }

    pre = fn
      # Store the alias
      {:alias, _, args} = expr, state ->
        {alias_from, alias_to} =
          case args do
            [{:__aliases__, _, from}] -> {from, List.last(from)}
            # Alias to must not be a nested name
            [{:__aliases__, _, from}, [as: {:__aliases__, _, [to]}]] -> {from, to}
          end

        alias_table = Map.put(state.alias_table, alias_to, alias_from)
        {expr, %{state | alias_table: alias_table}}

      {:require, _, [{:__aliases__, _, names}]} = expr, state ->
        unalias_name = expand_alias(names, state.alias_table)
        state = if unalias_name == macro_module, do: %{state | is_require: true}, else: state
        {expr, state}

      {:import, _, [{:__aliases__, _, names}]} = expr, state ->
        unalias_name = expand_alias(names, state.alias_table)
        state = if unalias_name == macro_module, do: %{state | is_import: true}, else: state
        {expr, state}

      # Going in module def
      {:defmodule, _, [{:__aliases__, _, names}, _]} = expr, state ->
        {expr, %{state | current_module: state.current_module ++ names}}

      # We are looking for dot construct like A.A1.macro()
      {{:., _, [module, accessor]}, _, args} = expr, state ->
        {:__aliases__, _, names} = module
        unalias_name = expand_alias(names, state.alias_table)

        # We must ensure both the name and the arity of the macro match, also the module
        # must be require beforehand
        state =
          if (state.is_require or state.is_import) and unalias_name == macro_module and
               {accessor, length(args)} in macros,
             do: Map.update!(state, :result, &(&1 ++ [expr])),
             else: state

        {expr, state}

      # or directly invoke macro() (through import)
      {variable, _, args} = expr, state when is_atom(variable) ->
        args_length = if args, do: length(args), else: 0

        state =
          if state.is_import and {variable, args_length} in macros,
            do: Map.update!(state, :result, &(&1 ++ [expr])),
            else: state

        {expr, state}

      expr, acc ->
        {expr, acc}
    end

    post = fn
      # Going out of module def
      {:defmodule, _, [{:__aliases__, _, names}, _]} = expr, state ->
        current_module =
          Enum.slice(state.current_module, 0, length(state.current_module) - length(names))

        {expr, %{state | current_module: current_module}}

      expr, acc ->
        {expr, acc}
    end

    {_, state} = Macro.traverse(quoted, initial_state, pre, post)
    state.result
  end

  @spec compile_invocation_exprs(ExCompileGraph.file_path(), atom()) :: [Macro.t()]
  @doc """
  Given a source file and a module, return all invocation of module functions during compile-time in
  the source file
  """
  def compile_invocation_exprs(source_file, sink_module) do
    {:ok, bin} = File.read(source_file)
    {:ok, quoted} = Code.string_to_quoted(bin)
    functions = sink_module.__info__(:functions)

    initial_state = %{
      alias_table: %{},
      is_import: false,
      is_require: false,
      current_module: [],
      result: []
    }

    pre = fn
      # Store the alias
      {:alias, _, args} = expr, state ->
        {alias_from, alias_to} =
          case args do
            [{:__aliases__, _, from}] -> {from, List.last(from)}
            # Alias to must not be a nested name
            [{:__aliases__, _, from}, [as: {:__aliases__, _, [to]}]] -> {from, to}
          end

        alias_table = Map.put(state.alias_table, alias_to, alias_from)
        {expr, %{state | alias_table: alias_table}}

      # {:require, _, [{:__aliases__, _, names}]} = expr, state ->
      #   unalias_name = expand_alias(names, state.alias_table)
      #   state = if unalias_name == module, do: %{state | is_require: true}, else: state
      #   {expr, state}

      {:import, _, [{:__aliases__, _, names}]} = expr, state ->
        unalias_name = expand_alias(names, state.alias_table)
        state = if unalias_name == sink_module, do: %{state | is_import: true}, else: state
        {expr, state}

      # Going in module def
      {:defmodule, _, [{:__aliases__, _, names}, _]} = expr, state ->
        {expr, %{state | current_module: state.current_module ++ names}}

      # We are looking for dot construct like A.A1.macro()
      {{:., _, [module, accessor]}, _, args} = expr, state ->
        {:__aliases__, _, names} = module
        unalias_name = expand_alias(names, state.alias_table)
        args_length = if args, do: length(args), else: 0

        # We must ensure both the name and the arity of the macro match, also the module
        # must be require beforehand
        state =
          if unalias_name == sink_module and {accessor, args_length} in functions,
            do: Map.update!(state, :result, &(&1 ++ [expr])),
            else: state

        {expr, state}

      {:def, _, _}, state ->
        {nil, state}

      # or directly invoke macro() (through import)
      {variable, _, args} = expr, state when is_atom(variable) ->
        state =
          if state.is_import and {variable, length(args)} in functions,
            do: Map.update!(state, :result, &(&1 ++ [expr])),
            else: state

        {expr, state}

      expr, acc ->
        {expr, acc}
    end

    post = fn
      # Going out of module def
      {:defmodule, _, [{:__aliases__, _, names}, _]} = expr, state ->
        current_module =
          Enum.slice(state.current_module, 0, length(state.current_module) - length(names))

        {expr, %{state | current_module: current_module}}

      expr, acc ->
        {expr, acc}
    end

    {_, state} = Macro.traverse(quoted, initial_state, pre, post)
    state.result
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

  # Functions/macros invocations or property accesses
  def expr_lines_span({{:., _, _}, context, _}) do
    start = context[:line]
    {start, start}
  end

  def expr_lines_span({variable, context, _}) when is_atom(variable) do
    start = context[:line]
    {start, start}
  end
end
