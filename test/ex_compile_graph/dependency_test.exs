defmodule ExCompileGraph.DependencyTest do
  use ExUnit.Case
  alias ExCompileGraph.{Graph, Dependency, TestUtils}

  setup_all context do
    sources_set_1 = [
      {
        """
        defmodule Direct.A1 do
          require Direct.A2

          Direct.A2.define()
        end
        """,
        "lib/direct/A1.ex"
      },
      {
        """
        defmodule Direct.A2 do
          defmacro define do
            Direct.A3.x()

            quote do
              def print do
                IO.puts("Hello world !!!")
              end
            end
          end
        end
        """,
        "lib/direct/A2.ex"
      },
      {
        """
        defmodule Direct.A3 do
          def x(), do: 1

          def y(), do: %Direct.A4{}
        end
        """,
        "lib/direct/A3.ex"
      },
      {
        """
        defmodule Direct.A4 do
          defstruct [:a, :b, :c, :d]

          def x, do: 1
        end
        """,
        "lib/direct/A4.ex"
      }
    ]

    sources_set_2 = [
      {
        """
        defmodule Transitive.A1 do
          def x(), do: Transitive.A2.x()
        end
        """,
        "lib/transitive/A1.ex"
      },
      {
        """
        defmodule Transitive.A2 do
          def x(), do: Transitive.A3.x()
        end
        """,
        "lib/transitive/A2.ex"
      },
      {
        """
        defmodule Transitive.A3 do
          def x(), do: 1
        end
        """,
        "lib/transitive/A3.ex"
      },
      {
        """
        defmodule Transitive.B1 do
          import Transitive.B2
        end
        """,
        "lib/transitive/B1.ex"
      },
      {
        """
        defmodule Transitive.B2 do
          import Transitive.B3
        end
        """,
        "lib/transitive/B2.ex"
      },
      {
        """
        defmodule Transitive.B3 do
          def x(), do: 1
        end
        """,
        "lib/transitive/B3.ex"
      },
      {
        """
        defmodule Transitive.C1 do
          require Transitive.C2

          Transitive.C2.x()
        end
        """,
        "lib/transitive/C1.ex"
      },
      {
        """
        defmodule Transitive.C2 do
          require Transitive.C3

          Transitive.C3.x()

          def x(), do: 1
        end
        """,
        "lib/transitive/C2.ex"
      },
      {
        """
        defmodule Transitive.C3 do
          def x(), do: 1
        end
        """,
        "lib/transitive/C3.ex"
      }
    ]

    sources_set_3 = [
      {
        """
        defmodule Recompile.A1 do
          Recompile.A2.x()
        end
        """,
        "lib/recompile/A1.ex"
      },
      {
        """
        defmodule Recompile.A2 do
          Recompile.A3.x()

          def x(), do: 1
        end
        """,
        "lib/recompile/A2.ex"
      },
      {
        """
        defmodule Recompile.A3 do
          def x(), do: 1
        end
        """,
        "lib/recompile/A3.ex"
      },
      {
        """
        defmodule Recompile.B1 do
          import Recompile.B2
        end
        """,
        "lib/recompile/B1.ex"
      },
      {
        """
        defmodule Recompile.B2 do
          import Recompile.B3
        end
        """,
        "lib/recompile/B2.ex"
      },
      {
        """
        defmodule Recompile.B3 do
          def x(), do: 1
        end
        """,
        "lib/recompile/B3.ex"
      },
      {
        """
        defmodule Recompile.C1 do
          Recompile.C2.x()
        end
        """,
        "lib/recompile/C1.ex"
      },
      {
        """
        defmodule Recompile.C2 do
          def x(), do: Recompile.C3.x()
        end
        """,
        "lib/recompile/C2.ex"
      },
      {
        """
        defmodule Recompile.C3 do
          def x(), do: Recompile.C4.x()
        end
        """,
        "lib/recompile/C3.ex"
      },
      {
        """
        defmodule Recompile.C4 do
          def x(), do: 1
        end
        """,
        "lib/recompile/C4.ex"
      },
      {
        """
        defmodule Recompile.D1 do
          import Recompile.D2
        end
        """,
        "lib/recompile/D1.ex"
      },
      {
        """
        defmodule Recompile.D2 do
          import Recompile.D3
        end
        """,
        "lib/recompile/D2.ex"
      },
      {
        """
        defmodule Recompile.D3 do
          Recompile.D4.x()
        end
        """,
        "lib/recompile/D3.ex"
      },
      {
        """
        defmodule Recompile.D4 do
          def x(), do: 1
        end
        """,
        "lib/recompile/D4.ex"
      }
    ]

    sources_set_4 = [
      {
        """
        defmodule Cause.A1 do
          import Cause.A2

          defmodule Nested do
            import Cause.A2
          end

          def x(), do: %Cause.A3{}
        end
        """,
        "lib/cause/A1.ex"
      },
      {
        """
        defmodule Cause.A2 do
          def x(), do: 1

          def y(), do: %Cause.A3{}

          def z() do
            case 1 > 2 do
              true -> 1 + 1
              false ->
                %Cause.A3{a: 1, b: 2}
            end
          end
        end
        """,
        "lib/cause/A2.ex"
      },
      {
        """
        defmodule Cause.A3 do
          defstruct [:a, :b]

          def x(), do: 1
        end
        """,
        "lib/cause/A3.ex"
      }
    ]

    setup_sources(sources_set_1 ++ sources_set_2 ++ sources_set_3 ++ sources_set_4, context)
  end

  describe "ExCompileGraph.Dependency.find_source_files/3" do
    # Use source_set_1
    test "Direct runtime references", %{graph: graph} do
      assert [] =
               Dependency.find_source_files(graph, "lib/direct/A1.ex", :runtime,
                 direct_only?: true
               )
               |> to_list()

      assert [] =
               Dependency.find_source_files(graph, "lib/direct/A2.ex", :runtime,
                 direct_only?: true
               )
               |> to_list()

      assert ["lib/direct/A2.ex"] =
               Dependency.find_source_files(graph, "lib/direct/A3.ex", :runtime,
                 direct_only?: true
               )
               |> to_list()

      assert ["lib/direct/A3.ex"] =
               Dependency.find_source_files(graph, "lib/direct/A4.ex", :runtime,
                 direct_only?: true
               )
               |> to_list()
    end

    # Use source_set_1
    test "Direct export references", %{graph: graph} do
      assert [] =
               ExCompileGraph.Dependency.find_source_files(graph, "lib/direct/A1.ex", :exports,
                 direct_only?: true
               )
               |> to_list()

      assert ["lib/direct/A1.ex"] =
               ExCompileGraph.Dependency.find_source_files(graph, "lib/direct/A2.ex", :exports,
                 direct_only?: true
               )
               |> to_list()

      assert [] =
               ExCompileGraph.Dependency.find_source_files(graph, "lib/direct/A3.ex", :exports,
                 direct_only?: true
               )
               |> to_list()

      assert ["lib/direct/A3.ex"] =
               ExCompileGraph.Dependency.find_source_files(graph, "lib/direct/A4.ex", :exports)
               |> to_list()
    end

    # Use source_set_1
    test "Direct compile references", %{graph: graph} do
      assert [] =
               ExCompileGraph.Dependency.find_source_files(graph, "lib/direct/A1.ex", :compile,
                 direct_only?: true
               )
               |> to_list()

      assert ["lib/direct/A1.ex"] =
               ExCompileGraph.Dependency.find_source_files(graph, "lib/direct/A2.ex", :compile,
                 direct_only?: true
               )
               |> to_list()

      assert [] =
               ExCompileGraph.Dependency.find_source_files(graph, "lib/direct/A3.ex", :compile,
                 direct_only?: true
               )
               |> to_list()

      assert [] =
               ExCompileGraph.Dependency.find_source_files(graph, "lib/direct/A4.ex", :compile,
                 direct_only?: true
               )
               |> to_list()
    end

    # Use source_set_2
    test "Transitive references", %{graph: graph} do
      assert ["lib/transitive/A1.ex", "lib/transitive/A2.ex"] =
               Dependency.find_source_files(
                 graph,
                 "lib/transitive/A3.ex",
                 :runtime
               )
               |> to_list()

      assert ["lib/transitive/B1.ex", "lib/transitive/B2.ex"] =
               Dependency.find_source_files(
                 graph,
                 "lib/transitive/B3.ex",
                 :exports
               )
               |> to_list()

      assert ["lib/transitive/C1.ex", "lib/transitive/C2.ex"] =
               Dependency.find_source_files(
                 graph,
                 "lib/transitive/C3.ex",
                 :compile
               )
               |> to_list()
    end
  end

  # Use source_set_3
  describe "ExCompileGraph.Dependency.recompile_dependencies/2" do
    test "Direct or transitive compile references", %{graph: graph} do
      assert ["lib/recompile/A1.ex", "lib/recompile/A2.ex"] =
               ExCompileGraph.Dependency.recompile_dependencies(graph, "lib/recompile/A3.ex")
               |> Map.get(:compile)
               |> to_list()
    end

    test "Direct exports references", %{graph: graph} do
      assert ["lib/recompile/B1.ex"] =
               ExCompileGraph.Dependency.recompile_dependencies(graph, "lib/recompile/B2.ex")
               |> Map.get(:exports)
               |> to_list()

      assert ["lib/recompile/B2.ex"] =
               ExCompileGraph.Dependency.recompile_dependencies(graph, "lib/recompile/B3.ex")
               |> Map.get(:exports)
               |> to_list()
    end

    test "Compile following by runtime references", %{graph: graph} do
      assert ["lib/recompile/C1.ex"] =
               ExCompileGraph.Dependency.recompile_dependencies(graph, "lib/recompile/C3.ex")
               |> Map.get(:compile_then_runtime)
               |> to_list()

      assert ["lib/recompile/C1.ex"] =
               ExCompileGraph.Dependency.recompile_dependencies(graph, "lib/recompile/C4.ex")
               |> Map.get(:compile_then_runtime)
               |> to_list()
    end

    test "Direct export following by compile references", %{graph: graph} do
      assert ["lib/recompile/D2.ex"] =
               ExCompileGraph.Dependency.recompile_dependencies(graph, "lib/recompile/D4.ex")
               |> Map.get(:exports_then_compile)
               |> to_list()
    end
  end

  # Use source_set_4
  describe "ExCompileGraph.xDependency.dependency_causes/1" do
    test "Exports dependency causes" do
      {:ok, manifest_path} = TestUtils.fixtures_manifest()

      assert [
               %Dependency.Cause{name: :import, lines_span: {2, 2}},
               %Dependency.Cause{name: :import, lines_span: {5, 5}}
             ] =
               Dependency.dependency_causes(%{
                 root_folder: "test/fixtures",
                 source_file: "lib/cause/A1.ex",
                 sink_file: "lib/cause/A2.ex",
                 manifest: manifest_path,
                 dependency_type: :exports
               })

      assert [
               %Dependency.Cause{name: :struct_usage, lines_span: {8, 8}}
             ] =
               Dependency.dependency_causes(%{
                 root_folder: "test/fixtures",
                 source_file: "lib/cause/A1.ex",
                 sink_file: "lib/cause/A3.ex",
                 manifest: manifest_path,
                 dependency_type: :exports
               })

      assert [
               %Dependency.Cause{name: :struct_usage, lines_span: {4, 4}},
               %Dependency.Cause{name: :struct_usage, lines_span: {10, 10}}
             ] =
               Dependency.dependency_causes(%{
                 root_folder: "test/fixtures",
                 source_file: "lib/cause/A2.ex",
                 sink_file: "lib/cause/A3.ex",
                 manifest: manifest_path,
                 dependency_type: :exports
               })
    end
  end

  defp to_list(set), do: set |> MapSet.to_list() |> Enum.sort()

  defp setup_sources(sources, context) do
    :ok = TestUtils.clear_fixtures()

    Enum.each(sources, fn {source, path} ->
      TestUtils.write_source(source, path)
    end)

    :ok = TestUtils.compile_fixtures()

    {:ok, manifest_path} = TestUtils.fixtures_manifest()
    graph = Graph.build(manifest_path)
    Map.put(context, :graph, graph)
  end
end
