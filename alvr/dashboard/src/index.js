import "bootstrap"
import "bootstrap/dist/css/bootstrap.css"
import "./style.css"
// import "../resources/style.css"
import { library, dom } from "@fortawesome/fontawesome-svg-core"
import {
    faPlug,
    faChartBar,
    faThLarge,
    faCog,
    faHdd,
    faThList,
    faInfoCircle,
    faGlobe,
    faPlus,
    faMinus,
    faTrash,
    faQuestionCircle,
    faUndo,
} from "@fortawesome/free-solid-svg-icons"

// Setup font-awesome
// Menu:
library.add(faPlug, faChartBar, faThLarge, faCog, faHdd, faThList, faInfoCircle, faGlobe)
// Other:
library.add(faPlus, faMinus, faTrash, faQuestionCircle, faUndo)
dom.watch()

// WASM entry point
import("../pkg/index.js").catch(console.error)
