import React from "react"
import ReactDOM from "react-dom"
import { useAsync } from "react-async-hook"
import { initializeSessionManager, SessionContextWrapper } from "./sessionManager"
import { Dashboard } from "./Dashboard"

function AsyncLoader(): JSX.Element {
    const future = useAsync(initializeSessionManager, [])

    return (
        <>
            {future.result && (
                <SessionContextWrapper initialSession={future.result[1]}>
                    <Dashboard settingsSchema={future.result[0]} />
                </SessionContextWrapper>
            )}
        </>
    )
}

// setup entry point
document.body.innerHTML += `<div id="root"></div>`

ReactDOM.render(
    <React.StrictMode>
        <AsyncLoader />
    </React.StrictMode>,
    document.getElementById("root"),
)

// Hot reload
if (module.hot) {
    module.hot.accept()
}
