import React from "react"
import ReactDOM from "react-dom"
import App from "./App"
import TRANSLATION from "./Translation"

TRANSLATION.compile("default").finally(() => {
    ReactDOM.render(
        <React.StrictMode>
            <App />
        </React.StrictMode>,
        document.getElementById("root"),
    )
})

// Hot reload
if (import.meta.hot) {
    import.meta.hot.accept()
}
