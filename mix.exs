defmodule ExCompileGraph.MixProject do
  use Mix.Project

  def project do
    [
      app: :ex_compile_graph,
      version: "0.1.0",
      elixir: "~> 1.11",
      start_permanent: Mix.env() == :prod,
      elixirc_paths: elixirc_paths(Mix.env()),
      deps: deps()
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      mod: {ExCompileGraph.Application, []},
      extra_applications: [:logger, :cowboy]
    ]
  end

  defp elixirc_paths(:test), do: ["lib", "test/support", "test/fixtures/lib"]
  defp elixirc_paths(_), do: ["lib"]

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:cowboy, "~> 2.0"},
      {:plug, "~> 1.14"},
      {:plug_cowboy, "~> 2.4"},
      {:jason, "~> 1.0"}
    ]
  end
end
