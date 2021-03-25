const httpProxy = require("http-proxy")
const proxy = httpProxy.createServer({ target: "http://localhost:8082" })

module.exports = {
    mount: {
        src: "/",
        static: {
            url: "/",
            static: true,
            resolve: false,
        },
    },
    plugins: ["@snowpack/plugin-react-refresh", "@snowpack/plugin-typescript"],
    routes: [
        {
            src: "/api/.*",
            dest: (req, res) => proxy.web(req, res),
        },
    ],
}
