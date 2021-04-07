const HtmlWebpackPlugin = require("html-webpack-plugin")
const CopyPlugin = require("copy-webpack-plugin")
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin")

module.exports = {
    mode: "production",
    entry: { index: "./src/index.js" },
    devtool: "eval-cheap-source-map",
    module: {
        rules: [{ test: /\.css$/, use: ["style-loader", "css-loader"] }],
    },
    plugins: [
        new HtmlWebpackPlugin({ title: "ALVR dashboard", favicon: "resources/favicon.png" }),
        new CopyPlugin({ patterns: [{ from: "resources/languages", to: "languages" }] }),
        new WasmPackPlugin({ crateDirectory: __dirname }),
    ],
    devServer: {
        hot: true,
        proxy: {
            "/api/events": { target: "ws://localhost:8082", ws: true },
            "/api/log": { target: "ws://localhost:8082", ws: true },
            "/api": "http://localhost:8082",
        },
    },
    experiments: {
        asyncWebAssembly: true,
    },
}
