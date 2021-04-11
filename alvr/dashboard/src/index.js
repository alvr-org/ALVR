import "bootstrap"
import "bootstrap/dist/css/bootstrap.min.css"
import "../resources/style.css"
import { library, dom } from "@fortawesome/fontawesome-svg-core"
import { faPlus, faMinus } from "@fortawesome/free-solid-svg-icons"

library.add(faPlus, faMinus)
dom.watch()

import("../pkg/index.js").catch(console.error)
