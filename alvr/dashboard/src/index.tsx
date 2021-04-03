import React, { Suspense, useState } from "react"
import ReactDOM from "react-dom"
import { useAsync } from "react-async-hook"
import {
    initializeSessionManager,
    SessionContextWrapper,
    SessionSettingsSection,
    SettingsSchema,
    useSession,
} from "./sessionManager"
import { Dashboard } from "./Dashboard"
import { useTranslation } from "react-i18next"
import "./translation"

function TranslationLoader({ schema }: { schema: SettingsSchema }): JSX.Element {
    const { i18n } = useTranslation()
    const session = useSession()

    const newLanguage = (session.session_settings["extra"] as SessionSettingsSection)[
        "language"
    ] as string

    const [prevLanguage, setPrevLanguage] = useState(newLanguage)

    useAsync(async () => {
        if (newLanguage !== prevLanguage) {
            await i18n.changeLanguage(newLanguage !== "" ? newLanguage : undefined)
            setPrevLanguage(newLanguage)
        }
    }, [i18n, session])

    return <Dashboard settingsSchema={schema} />
}

function SessionLoader() {
    const future = useAsync(initializeSessionManager, [])

    return (
        <>
            {future.result && (
                <SessionContextWrapper initialSession={future.result[1]}>
                    <Suspense fallback="">
                        <TranslationLoader schema={future.result[0]} />
                    </Suspense>
                </SessionContextWrapper>
            )}
        </>
    )
}

// setup entry point
document.body.innerHTML += `<div id="root"></div>`

ReactDOM.render(
    <React.StrictMode>
        <SessionLoader />
    </React.StrictMode>,
    document.getElementById("root"),
)

// Hot reload
if (module.hot) {
    module.hot.accept()
}
