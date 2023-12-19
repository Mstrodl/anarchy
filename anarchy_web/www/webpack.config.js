const CopyWebpackPlugin = require("copy-webpack-plugin");
const MonacoWebpackPlugin = require("monaco-editor-webpack-plugin");
const path = require("path");
const MONACO_DIR = path.resolve(__dirname, "./node_modules/monaco-editor");

module.exports = {
  entry: "./index.ts",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "index.js",
  },
  mode: "development",
  plugins: [
    new CopyWebpackPlugin(["index.html"]),
    new MonacoWebpackPlugin({
      // available options are documented at https://github.com/microsoft/monaco-editor/blob/main/webpack-plugin/README.md#options
      languages: ["javascript"],
    }),
  ],
  module: {
    rules: [
      {
        test: /\.(?:js|mjs|cjs)$/,
        include: /monaco-editor/,
        use: {
          loader: "babel-loader",
          options: {
            //presets: [
            //  ['@babel/preset-env', { targets: "defaults" }]
            //],
            plugins: ["@babel/plugin-transform-class-properties"],
          },
        },
      },
      {
        test: /\.tsx?$/,
        use: "ts-loader",
        exclude: /node_modules/,
      },
      // {
      //   test: /\.wasm$/,
      //   type: "webassembly/experimental",
      // },
      {
        test: /\.css$/,
        use: ["style-loader", "css-loader"],
      },
    ],
  },
  resolve: {
    extensions: [".tsx", ".ts", ".js", ".wasm"],
  },
  experiments: {
    asyncWebAssembly: true,
  },
  devServer: {
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
  },
};
