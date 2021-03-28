/* eslint-disable no-undef */

const HtmlWebpackPlugin = require("html-webpack-plugin")

module.exports = {
    mode: "development",
    entry: "./src/index.tsx",
    target: "web",
    resolve: {
        extensions: [".ts", ".tsx", ".js"],
    },
    module: {
        rules: [
            {
                test: /\.tsx?$/,
                use: "ts-loader",
                exclude: /node_modules/,
            },
            {
                test: /\.css$/,
                use: ["style-loader", "css-loader"],
            },
        ],
    },
    plugins: [new HtmlWebpackPlugin({ title: "ALVR dashboard" })],
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
            chunks: "all"
        },
    },
}
