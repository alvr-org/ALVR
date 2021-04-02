import React, { Suspense } from "react"
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

    const language = (session.session_settings["extra"] as SessionSettingsSection)[
        "locale"
    ] as string

    const future = useAsync(async () => {
        console.error(language != "" ? language : undefined)
        await i18n.changeLanguage(language != "" ? language : undefined)
    }, [i18n, session])

    return <> {!future.loading && <Dashboard settingsSchema={schema} />}</>
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

// function BlockingDashboardLoader({ schema }: { schema: SettingsSchema }): JSX.Element {
//     const { i18n } = useTranslation()
//     const session = useSession()

//     useEffect(() => {
//         const language = (session.session_settings["extra"] as SessionSettingsSection)[
//             "locale"
//         ] as string

//         i18n.changeLanguage(language ?? undefined) // async
//     }, [i18n, session])

//     return <Dashboard settingsSchema={schema} />
// }

// function BlockingSessionLoader(): JSX.Element {
//     let schema: SettingsSchema | null = null
//     let session: Session | null = null

//     ;(async () => {
//         const pair = await initializeSessionManager()
//         schema = pair[0]
//         session = pair[1]
//     })()

//     while(!schema )

//     return (
//         <SessionContextWrapper initialSession={session}>
//             <Suspense fallback={<Spin />}>
//                 <BlockingDashboardLoader schema={schema} />
//             </Suspense>
//         </SessionContextWrapper>
//     )
// }
