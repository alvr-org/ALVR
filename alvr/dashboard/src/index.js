import "bootstrap"
import "bootstrap/dist/css/bootstrap.css"
// import "../resources/style.css"
import { library, dom } from "@fortawesome/fontawesome-svg-core"
import { faPlus, faMinus } from "@fortawesome/free-solid-svg-icons"

// Setup font-awesome
library.add(faPlus, faMinus)
dom.watch()

// WASM entry point
import("../pkg/index.js").catch(console.error)
