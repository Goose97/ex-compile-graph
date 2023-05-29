defmodule ExCompileGraph.TestUtils do
  @manifest "compile.elixir"
  @fixture_lib "test/fixtures"

  def write_source(source, path) do
    path = fixtures_path(path)
    Path.dirname(path) |> File.mkdir_p!()
    File.write!(path, source)
  end

  def fixtures_path(path), do: Path.join([@fixture_lib, path])

  defp fixtures_path, do: Path.join([File.cwd!(), "test", "fixtures"])

  def compile_fixtures() do
    ref = make_ref()

    exit_code =
      Mix.Shell.cmd("mix elixir.compile", [cd: fixtures_path()], fn output ->
        send(self(), {ref, output})
      end)

    if exit_code == 0, do: :ok, else: {:error, receive_stream_response(ref)}
  end

  def clear_fixtures() do
    with :ok <- clear_source_folder(),
         :ok <- clear_build_artifacts() do
      :ok
    end
  end

  defp clear_source_folder() do
    lib_folder = Path.join([@fixture_lib, "lib"])

    with {:ok, _} <- File.rm_rf(lib_folder),
         :ok <- File.mkdir_p(lib_folder) do
      :ok
    end
  end

  defp clear_build_artifacts() do
    ref = make_ref()

    exit_code =
      Mix.Shell.cmd("mix clean", [cd: fixtures_path()], fn output ->
        send(self(), {ref, output})
      end)

    if exit_code == 0, do: :ok, else: {:error, receive_stream_response(ref)}
  end

  def fixtures_manifest() do
    ref = make_ref()

    exit_code =
      Mix.Shell.cmd(
        ~s/mix run -e "Mix.Project.manifest_path() |> Mix.Shell.IO.info()"/,
        [cd: fixtures_path()],
        fn output -> send(self(), {ref, output}) end
      )

    receive do
      {^ref, output} ->
        if exit_code == 0,
          do: {:ok, Path.join([String.trim(output), @manifest])},
          else: {:error, output}
    after
      0 ->
        raise RuntimeError,
          message:
            "#{__MODULE__}.fixtures_manifest: expect to receive a message, instead got none"
    end
  end

  defp receive_stream_response(ref, acc \\ "") do
    receive do
      {^ref, output} ->
        receive_stream_response(ref, acc <> output)
    after
      0 ->
        acc
    end
  end
end
