import React, { createContext, useContext } from "react"
import i18next from "i18next"
import I18nextBrowserLanguageDetector from "i18next-browser-languagedetector"
import I18NextHttpBackend from "i18next-http-backend"
import { initReactI18next, useTranslation } from "react-i18next"

i18next
    .use(I18NextHttpBackend)
    .use(I18nextBrowserLanguageDetector)
    .use(initReactI18next)
    .init({
        fallbackLng: "en-US",
        debug: true,
        returnObjects: false,
        detection: {
            order: ["navigator", "localStorage"],
        },
        interpolation: {
            escapeValue: false,
        },
        backend: {
            loadPath: "/locales/{{lng}}.json",
        },
    })
export default i18next

const TransContext = createContext<string[]>([])

export function Trans(props: { children: React.ReactNode; node: string }): JSX.Element {
    const routeSegments = [...useContext(TransContext), props.node]

    return <TransContext.Provider value={routeSegments}>{props.children}</TransContext.Provider>
}

export interface TransKeys {
    name: string | string[]
    help?: string
    notice?: string
}

export function useTransKeys(subkey?: string): TransKeys {
    const { i18n } = useTranslation()

    let routeSegments = useContext(TransContext)
    if (subkey) {
        routeSegments = [...routeSegments, subkey]
    }
    const route = routeSegments.join(".")

    if (i18n.exists(route) && !i18n.exists(route + ".name")) {
        return { name: route }
    } else {
        let name: string | string[]
        if (i18n.exists(route + ".name")) {
            name = route + ".name"
        } else {
            name = routeSegments
        }

        let help: string | undefined = undefined
        if (i18n.exists(route + ".help")) {
            help = route + ".help"
        }

        let notice: string | undefined = undefined
        if (i18n.exists(route + ".notice")) {
            notice = route + ".notice"
        }

        return { name, help, notice }
    }
}

export interface TransValues {
    name: string
    help?: string
    notice?: string
}

export function useTrans(subkey?: string): TransValues {
    const { t } = useTranslation()

    const { name, help, notice } = useTransKeys(subkey)

    return {
        name: typeof name === "string" ? t(name) : name[name.length - 1],
        help: help && t(help),
        notice: notice && t(notice),
    }
}

export function TransName({ subkey }: { subkey: string }): JSX.Element {
    const { name } = useTrans(subkey)
    return <>{name}</>
}
