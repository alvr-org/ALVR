export const MAX_LINES_COUNT = 50

export enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
}

type ReadonlyLogBuffer = readonly { timestamp: string; level: LogLevel; message: string }[]
type LogListener = (buffer: ReadonlyLogBuffer) => void

const buffer: { timestamp: string; level: LogLevel; message: string }[] = []
let listener: (buffer: ReadonlyLogBuffer) => void = () => {}
let websocket: WebSocket | null = null

function storeLogLine(line: string) {
    const [timestamp, levelString, message] = line.split(/ (?! ) (.*)/)

    let level: LogLevel
    if (levelString === "[ERROR]") {
        level = LogLevel.Error
    } else if (levelString === "[WARN]") {
        level = LogLevel.Warning
    } else if (levelString === "[INFO]") {
        level = LogLevel.Info
    } else {
        level = LogLevel.Debug
    }

    buffer.push({ timestamp, level, message })

    if (buffer.length > MAX_LINES_COUNT) {
        buffer.shift()
    }
}

function resetWebsocket(): void {
    websocket = new WebSocket(`ws://${window.location.host}/events`)

    websocket.onmessage = ev => {
        storeLogLine(ev.data)
        listener(buffer)
    }

    websocket.onerror = ev => console.error("EventDispatcher error:", ev)

    websocket.onclose = () => {
        console.info("Log websocket closed. Reopening...")
        resetWebsocket()
    }
}

resetWebsocket()

export function subscribeToLog(callback: LogListener): ReadonlyLogBuffer {
    listener = callback

    return buffer
}
