/* eslint-disable @typescript-eslint/no-var-requires */
/* eslint-disable no-undef */

const HtmlWebpackPlugin = require("html-webpack-plugin")
const CopyPlugin = require("copy-webpack-plugin")
const ReactRefreshWebpackPlugin = require("@pmmmwh/react-refresh-webpack-plugin")
const { HotModuleReplacementPlugin } = require("webpack")
const ForkTsCheckerWebpackPlugin = require("fork-ts-checker-webpack-plugin")

const theme = require("./resources/theme")

module.exports = (_, argv) => {
    const mode = argv.mode
    const isDevelopment = mode !== "production"
    return {
        mode: mode || "development",
        entry: "./src/index.tsx",
        target: "web",
        resolve: {
            extensions: [".ts", ".tsx", ".js"],
        },
        devtool: isDevelopment ? "eval-cheap-source-map" : false,
        module: {
            rules: [
                {
                    test: /\.[jt]sx?$/,
                    exclude: /node_modules/,
                    use: [
                        {
                            loader: require.resolve("babel-loader"),
                            options: {
                                plugins: [
                                    isDevelopment && require.resolve("react-refresh/babel"),
                                ].filter(Boolean),
                            },
                        },
                    ],
                },
                {
                    test: /\.css$/,
                    use: ["style-loader", "css-loader"],
                },
                {
                    test: /\.less/,
                    use: [
                        "style-loader",
                        "css-loader",
                        {
                            loader: "less-loader",
                            options: {
                                lessOptions: {
                                    modifyVars: {
                                        ...theme,
                                    },
                                    javascriptEnabled: true,
                                },
                            },
                        },
                    ],
                },
            ],
        },
        plugins: [
            new HtmlWebpackPlugin({ title: "ALVR dashboard", favicon: "resources/favicon.png" }),
            new CopyPlugin({ patterns: [{ from: "resources/locales", to: "locales" }] }),
            isDevelopment && new HotModuleReplacementPlugin(),
            isDevelopment && new ReactRefreshWebpackPlugin(),
            new ForkTsCheckerWebpackPlugin(),
        ].filter(Boolean),
        devServer: {
            hot: true,
            proxy: {
                "/api/events": { target: "ws://localhost:8082", ws: true },
                "/api/log": { target: "ws://localhost:8082", ws: true },
                "/api": "http://localhost:8082",
            },
        },
        optimization: {
            splitChunks: {
                maxSize: 250000,
                chunks: "all",
            },
        },
    }
}
