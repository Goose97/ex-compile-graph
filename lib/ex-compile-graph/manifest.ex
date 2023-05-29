defmodule ExCompileGraph.Manifest do
  @moduledoc """
  Module contains utilities to quickly query the manifest file
  """

  @spec build_lookup_table(ExCompileGraph.file_path()) :: :ets.tid()
  def build_lookup_table(manifest) do
    table_ref = :ets.new(Module.concat(__MODULE__, LookupTable), [:set])
    {modules, sources} = Mix.Compilers.Elixir.read_manifest(manifest)

    Enum.each(modules, fn module ->
      struct = ExCompileGraph.Module.from_record(module)
      :ets.insert(table_ref, {struct.module, struct, :module})
    end)

    Enum.each(sources, fn source ->
      struct = ExCompileGraph.SourceFile.from_record(source)
      :ets.insert(table_ref, {struct.path, struct, :source_file})
    end)

    table_ref
  end

  def delete_lookup_table(table_ref), do: :ets.delete(table_ref)

  @spec lookup_module(:ets.tid(), atom()) :: {:ok, ExCompileGraph.Module.t()} | {:error, atom()}
  def lookup_module(table_ref, module) when is_atom(module) do
    case :ets.lookup(table_ref, module) do
      [{_, struct, :module}] -> {:ok, struct}
      _ -> {:error, :not_found}
    end
  end

  @spec lookup_module!(:ets.tid(), atom()) :: ExCompileGraph.Module.t()
  def lookup_module!(table_ref, module) when is_atom(module) do
    case lookup_module(table_ref, module) do
      {:ok, struct} ->
        struct

      {:error, error} ->
        raise RuntimeError,
          message: "#{__MODULE__}.lookup_module!: encounter error #{inspect(error)}"
    end
  end

  @spec lookup_source_file!(:ets.tid(), binary()) :: ExCompileGraph.SourceFile.t()
  def lookup_source_file!(table_ref, path) when is_binary(path) do
    [{_, struct, :source_file}] = :ets.lookup(table_ref, path)
    struct
  end

  def all_modules(table_ref) do
    # Compiled from :ets.fun2ms(fn {_, struct, :module} -> struct end)
    match_spec = [{{:_, :"$1", :module}, [], [:"$1"]}]
    :ets.select(table_ref, match_spec)
  end

  def all_source_files(table_ref) do
    # Compiled from :ets.fun2ms(fn {_, struct, :source_file} -> struct end)
    match_spec = [{{:_, :"$1", :source_file}, [], [:"$1"]}]
    :ets.select(table_ref, match_spec)
  end
end
