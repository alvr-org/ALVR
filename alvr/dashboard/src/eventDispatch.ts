const listeners: Record<string, (data: unknown) => void> = {}
let websocket: WebSocket | null = null

function resetWebsocket(): void {
    websocket = new WebSocket(`ws://${window.location.host}/api/events`)

    websocket.onmessage = msgEv => {
        const event: { id: string; data: unknown } = JSON.parse(msgEv.data)

        const maybeCallback = listeners[event.id]
        maybeCallback?.(event.data)
    }

    websocket.onerror = ev => console.error("EventDispatcher error:", ev)

    websocket.onclose = () => {
        console.info("Event websocket closed. Reopening...")
        resetWebsocket()
    }
}

resetWebsocket()

export function subscribeToEvent<T>(id: string, callback: (data: T) => void): void {
    listeners[id] = callback as (data: unknown) => void
}
