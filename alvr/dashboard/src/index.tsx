import React from "react"
import ReactDOM from "react-dom"
import { useAsync } from "react-async-hook"
import { initializeSessionManager } from "./sessionManager"
import { Dashboard } from "./Dashboard"

function AsyncLoader(): JSX.Element {
    const futureSession = useAsync(initializeSessionManager, [])

    return <>{futureSession.result && <Dashboard initialSession={futureSession.result} />}</>
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
