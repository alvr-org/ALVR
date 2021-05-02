import "./style.css"
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
    faSpinner,
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
library.add(faSpinner, faPlus, faMinus, faTrash, faQuestionCircle, faUndo)
dom.watch()

// WASM entry point
import("../pkg/index.js").catch(console.error)
