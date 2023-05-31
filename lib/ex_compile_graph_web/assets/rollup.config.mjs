import babel from "@rollup/plugin-babel";
import commonjs from "@rollup/plugin-commonjs";
import { nodeResolve } from "@rollup/plugin-node-resolve";
import postcss from "rollup-plugin-postcss";
import postcssImport from "postcss-import";
import replace from "@rollup/plugin-replace";
import typescript from "@rollup/plugin-typescript";

export default {
  input: "src/index.tsx",
  output: {
    dir: "../../../priv/static",
    format: "iife",
  },
  plugins: [
    nodeResolve(),
    babel({ babelHelpers: "bundled" }),
    commonjs(),
    replace({
      "process.env.NODE_ENV": JSON.stringify("production"),
    }),
    postcss({
      plugins: [postcssImport],
    }),
    typescript({ tsconfig: "./tsconfig.json" }),
  ],
};
