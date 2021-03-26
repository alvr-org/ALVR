import React from "react"
import ReactDOM from "react-dom"
import { useAsync } from "react-async-hook"
import { initializeSessionManager } from "./sessionManager"
import { Dashboard } from "./Dashboard"

function AsyncLoader(): JSX.Element {
    const future = useAsync(initializeSessionManager, [])

    return <>{future.result && <Dashboard initialSession={future.result[1]} />}</>
}

ReactDOM.render(
    <React.StrictMode>
        <AsyncLoader />
    </React.StrictMode>,
    document.getElementById("root"),
)

// Hot reload
if (import.meta.hot) {
    import.meta.hot.accept()
}
