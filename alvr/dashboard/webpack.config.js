/* eslint-disable @typescript-eslint/no-var-requires */
/* eslint-disable no-undef */

const HtmlWebpackPlugin = require("html-webpack-plugin")
const CopyPlugin = require("copy-webpack-plugin")
const ReactRefreshWebpackPlugin = require("@pmmmwh/react-refresh-webpack-plugin")

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
                    test: /\.tsx?$/,
                    loader: "ts-loader",
                    exclude: /node_modules/,
                    options: {
                        getCustomTransformers: () => ({
                            before: [isDevelopment && require("react-refresh-typescript")()].filter(
                                Boolean,
                            ),
                        }),
                    },
                },
                {
                    test: /\.css$/,
                    use: ["style-loader", "css-loader"],
                },
            ],
        },
        plugins: [
            new HtmlWebpackPlugin({
                title: "ALVR dashboard",
                favicon: "resources/favicon.png",
            }),
            new CopyPlugin({
                patterns: [
                    {
                        from: "resources/locales",
                        to: "locales",
                    },
                ],
            }),
            isDevelopment && new ReactRefreshWebpackPlugin(),
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
