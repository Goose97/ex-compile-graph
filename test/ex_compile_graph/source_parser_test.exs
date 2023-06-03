defmodule ExCompileGraph.SourceParserTest do
  use ExUnit.Case
  alias ExCompileGraph.{SourceParser, TestUtils}
  import ExCompileGraph.TestUtils, only: [fixtures_path: 1]

  setup_all do
    sources_set_1 = [
      {
        """
        defmodule DefModule.A1 do
          def x(), do: 1
        end
        """,
        "lib/source_parser/A1.ex"
      },
      {
        """
        defmodule DefModule.A2 do
          def x(), do: 1
        end

        defmodule DefModule.A3 do
          def x(), do: 1
        end
        """,
        "lib/source_parser/A2.ex"
      },
      {
        """
        defmodule DefModule do
          def x(), do: 1

          defmodule Deeply do
            def x(), do: 2

            defmodule Nested do
              def x(), do: 3

              defmodule A4 do
                def x(), do: 4
              end
            end
          end
        end
        """,
        "lib/source_parser/A4.ex"
      }
    ]

    sources_set_2 = [
      {
        """
        defmodule Import.B1 do
          import Import.B2

          defmodule Nested do
            import Import.B2
          end
        end
        """,
        "lib/source_parser/B1.ex"
      },
      {
        """
        defmodule Import.B2 do
          def x(), do: 1
        end
        """,
        "lib/source_parser/B2.ex"
      },
      {
        """
        defmodule Import.B3 do
          def x() do
            import Import.B2

            1
          end
        end
        """,
        "lib/source_parser/B3.ex"
      }
    ]

    sources_set_3 = [
      {
        """
        defmodule StructDef.C1 do
          defstruct [:a, :b]
        end

        defmodule StructDef.C2 do
          defstruct [:a, :b]
        end
        """,
        "lib/source_parser/C1.ex"
      },
      {
        """
        defmodule StructDef.C3 do
          defstruct [:a, :b]

          defmodule Deeply do
            defstruct [:a, :b]

            defmodule Nested do
              defstruct [:a, :b]
            end
          end
        end
        """,
        "lib/source_parser/C3.ex"
      }
    ]

    sources_set_4 = [
      {
        """
        defmodule MacroExpr.D1 do
          require MacroExpr.D2
          import MacroExpr.D3

          def x(), do: MacroExpr.D2.x1() + MacroExpr.D2.x2()
          def y(), do: y1() + y2()
        end
        """,
        "lib/source_parser/D1.ex"
      },
      {
        """
        defmodule MacroExpr.D2 do
          defmacro x1() do
            quote do
              1 + 1
            end
          end

          defmacro x2() do
            quote do
              1 + 1
            end
          end
        end
        """,
        "lib/source_parser/D2.ex"
      },
      {
        """
        defmodule MacroExpr.D3 do
          defmacro y1() do
            quote do
              1 + 1
            end
          end

          defmacro y2() do
            quote do
              1 + 1
            end
          end
        end
        """,
        "lib/source_parser/D3.ex"
      },
      {
        """
        defmodule MacroExpr.D4 do
          require MacroExpr.D2
          require MacroExpr.D3
          alias MacroExpr.D3
          alias MacroExpr.D3, as: D3Aliased

          def x(), do: D3.y1()
          def x1(), do: D3Aliased.y2()
          def x2(), do: MacroExpr.D2.x1()
        end
        """,
        "lib/source_parser/D4.ex"
      },
      {
        """
        defmodule MacroExpr.D5 do
          import MacroExpr.D3
          def y1(x), do: x + 1

          # Should be arity-aware
          def x(), do: y1(1)

          def x1(), do: y1
        end
        """,
        "lib/source_parser/D5.ex"
      }
    ]

    sources_set_5 = [
      {
        """
        defmodule CompileTimeInvocation.E1 do
          alias CompileTimeInvocation.E2, as: E2Aliased
          CompileTimeInvocation.E2.x()

          E2Aliased.x(1)
        end
        """,
        "lib/source_parser/E1.ex"
      },
      {
        """
        defmodule CompileTimeInvocation.E2 do
          def x(), do: 1
          def x(x), do: x + 1
        end
        """,
        "lib/source_parser/E2.ex"
      }
    ]

    setup_sources(
      sources_set_1 ++ sources_set_2 ++ sources_set_3 ++ sources_set_4 ++ sources_set_5
    )
  end

  # Use sources set 1
  describe "ExCompileGraph.SourceParser.defmodule_expr/2" do
    test "Single module files" do
      assert {:ok, {:defmodule, _, _}} =
               SourceParser.defmodule_expr(
                 fixtures_path("lib/source_parser/A1.ex"),
                 DefModule.A1
               )
    end

    test "Multiple modules files" do
      assert {:ok, {:defmodule, _, _}} =
               SourceParser.defmodule_expr(
                 fixtures_path("lib/source_parser/A2.ex"),
                 DefModule.A2
               )

      assert {:ok, {:defmodule, _, _}} =
               ExCompileGraph.SourceParser.defmodule_expr(
                 fixtures_path("lib/source_parser/A2.ex"),
                 DefModule.A3
               )
    end

    test "Nested modules files" do
      assert {:ok, {:defmodule, _, _}} =
               ExCompileGraph.SourceParser.defmodule_expr(
                 fixtures_path("lib/source_parser/A4.ex"),
                 DefModule.Deeply.Nested
               )

      assert {:ok, {:defmodule, _, _}} =
               ExCompileGraph.SourceParser.defmodule_expr(
                 fixtures_path("lib/source_parser/A4.ex"),
                 DefModule.Deeply.Nested.A4
               )
    end

    test "Not found modules" do
      assert {:error, :not_found} =
               SourceParser.defmodule_expr(
                 fixtures_path("lib/source_parser/A2.ex"),
                 DefModule.A4
               )
    end
  end

  # Use sources set 2
  describe "ExCompileGraph.SourceParser.scan_module_exprs/3" do
    test "In module top-level" do
      assert [_] =
               SourceParser.scan_module_exprs(
                 fixtures_path("lib/source_parser/B1.ex"),
                 Import.B1,
                 :import
               )

      assert [_] =
               SourceParser.scan_module_exprs(
                 fixtures_path("lib/source_parser/B1.ex"),
                 Import.B1.Nested,
                 :import
               )
    end

    test "In module function definitions" do
      assert [_] =
               SourceParser.scan_module_exprs(
                 fixtures_path("lib/source_parser/B3.ex"),
                 Import.B3,
                 :import
               )
    end

    test "No import epxressions" do
      assert [] =
               SourceParser.scan_module_exprs(
                 fixtures_path("lib/source_parser/B2.ex"),
                 Import.B2,
                 :import
               )
    end
  end

  describe "ExCompileGraph.SourceParser.struct_expr/2" do
    test "With no alias" do
      ast =
        Code.string_to_quoted!("""
        defmodule A do
          defstruct [:a, :b]
        end

        defmodule B do
          def x(), do: %A{}

          def y() do
            if 2 > 1, do: %A{a: 1, b: 2}

            case 2 < 1 do
              true -> %A{a: 1}
              false -> nil
            end
          end
        end
        """)

      exprs = SourceParser.struct_expr(ast, [A])
      assert length(exprs) == 3
    end

    test "With aliases" do
      ast =
        Code.string_to_quoted!("""
        defmodule A.A1 do
          defstruct [:a, :b]
        end

        defmodule B do
          alias A.A1
          alias A.A1, as: A2

          def x(), do: %A1{}
          def y(), do: %A2{}
        end
        """)

      exprs = SourceParser.struct_expr(ast, [A.A1])
      assert length(exprs) == 2
    end
  end

  # Use sources set 3
  describe "ExCompileGraph.SourceParser.struct_defs/1" do
    test "With no nested modules" do
      assert [
               {StructDef.C2, _},
               {StructDef.C1, _}
             ] = SourceParser.struct_defs!(fixtures_path("lib/source_parser/C1.ex"))
    end

    test "With nested modules" do
      assert [
               {StructDef.C3.Deeply.Nested, _},
               {StructDef.C3.Deeply, _},
               {StructDef.C3, _}
             ] = SourceParser.struct_defs!(fixtures_path("lib/source_parser/C3.ex"))
    end
  end

  # Use sources_set_4
  describe "ExCompileGraph.SourceParser.macro_exprs/2" do
    test "With require" do
      exprs = SourceParser.macro_exprs(fixtures_path("lib/source_parser/D1.ex"), MacroExpr.D2)
      assert length(exprs) == 2
    end

    test "With import" do
      exprs = SourceParser.macro_exprs(fixtures_path("lib/source_parser/D1.ex"), MacroExpr.D3)
      assert length(exprs) == 2
    end

    test "With alias" do
      exprs = SourceParser.macro_exprs(fixtures_path("lib/source_parser/D4.ex"), MacroExpr.D3)
      assert length(exprs) == 2
    end

    test "Should be arity-aware" do
      exprs = SourceParser.macro_exprs(fixtures_path("lib/source_parser/D5.ex"), MacroExpr.D3)
      assert length(exprs) == 1
    end
  end

  # Use sources_set_5
  describe "ExCompileGraph.SourceParser.compile_invocation_exprs/2" do
    test "Function calls in module body" do
      exprs =
        ExCompileGraph.SourceParser.compile_invocation_exprs(
          fixtures_path("lib/source_parser/E1.ex"),
          CompileTimeInvocation.E2
        )

      assert length(exprs) == 2
    end
  end

  defp setup_sources(sources) do
    :ok = TestUtils.clear_fixtures()

    Enum.each(sources, fn {source, path} ->
      TestUtils.write_source(source, path)
    end)

    :ok = TestUtils.compile_fixtures()
  end
end
