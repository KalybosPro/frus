package dev.frus.intellij

/**
 * A stand-in for `frus run --machine`, run by the tests as a program of its own: it builds (on
 * standard error), starts, says each command it is given back as an application log line, and
 * ends on `app.stop`.
 */
object FakeFrus {
    @JvmStatic
    fun main(args: Array<String>) {
        System.err.println("   Compiling app v0.1.0")
        println("""{"event":"app.start","params":{"deviceName":"Fake device"}}""")
        println("""{"event":"app.log","params":{"log":"hello from the app"}}""")
        println("""{"event":"app.log","params":{"log":"an error","error":true}}""")
        println("""{"event":"app.started","params":{}}""")
        System.out.flush()
        while (true) {
            val line = readlnOrNull() ?: return
            val message = Json.parseOrNull(line) as? Map<*, *> ?: continue
            val method = message.str("method")
            println(Json.write(mapOf("event" to "app.log", "params" to mapOf("log" to "got $method ${Json.write(message["params"])}"))))
            if (method == "app.restart") {
                println("""{"event":"app.restarted","params":{"fullRestart":${message.obj("params")?.bool("fullRestart")},"seconds":1.25}}""")
            }
            System.out.flush()
            if (method == "app.stop") {
                println("""{"event":"app.stop","params":{}}""")
                return
            }
        }
    }
}
