package dev.frus.intellij

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File

class JsonTest {
    @Test
    fun readsTheDaemonsMessages() {
        val m = Json.parseOrNull("""{"event":"device.added","params":{"id":"emulator-5554","emulator":true,"n":1.5,"x":null,"l":[1,"a\"b\\né"]}}""") as Map<*, *>
        assertEquals("device.added", m.str("event"))
        val p = m.obj("params")!!
        assertEquals("emulator-5554", p.str("id"))
        assertTrue(p.bool("emulator"))
        assertEquals(1.5, p["n"])
        assertTrue(p.containsKey("x"))
        assertEquals(listOf(1.0, "a\"b\\né"), p["l"])
    }

    @Test
    fun aLineThatIsNotJsonIsNull() {
        assertNull(Json.parseOrNull("   Compiling frus v0.5.0"))
        assertNull(Json.parseOrNull("{\"a\":1"))
        assertNull(Json.parseOrNull("{\"a\":1} trailing"))
    }

    @Test
    fun writesOneLine() {
        assertEquals(
            """{"id":3,"method":"app.restart","params":{"fullRestart":false,"s":"a\"\nb"}}""",
            Json.write(linkedMapOf("id" to 3, "method" to "app.restart", "params" to linkedMapOf("fullRestart" to false, "s" to "a\"\nb"))),
        )
        assertEquals("[1,2.5,null,true]", Json.write(listOf(1.0, 2.5, null, true)))
    }

    @Test
    fun whatItWritesItReads() {
        val value = linkedMapOf("k" to listOf("\u0001\t", mapOf("z" to true)))
        assertEquals(value, Json.parseOrNull(Json.write(value)))
    }
}

class ProtocolTest {
    @get:Rule
    val temp = TemporaryFolder()

    private fun device(id: String, platform: String, category: String, emulator: Boolean = false, name: String = id) =
        Device(id, name, platform, category, "", emulator, true, null)

    @Test
    fun aDeviceFromTheDaemon() {
        val d = Device.from(mapOf("id" to "chrome", "name" to "Chrome", "platform" to "web", "category" to "web", "available" to false, "unavailableReason" to "not installed"))!!
        assertEquals("Chrome", d.name)
        assertFalse(d.available)
        assertEquals("not installed", d.unavailableReason)
        assertNull(Device.from(mapOf("name" to "no id")))
        assertTrue(Device.from(mapOf("id" to "x"))!!.available)
    }

    @Test
    fun simulatedDevicesHaveAnIdOfTheirOwn() {
        assertFalse(device("windows", "windows", "desktop").isSimulated)
        assertTrue(device("iphone-15", "windows", "desktop").isSimulated)
        assertFalse(device("pixel", "android", "mobile").isSimulated)
    }

    @Test
    fun emulatorsStoppedAndRunning() {
        assertTrue(device("Pixel_7", "android", "mobile", emulator = true).isStoppedEmulator)
        assertFalse(device("emulator-5554", "android", "mobile", emulator = true).isStoppedEmulator)
        assertTrue(device("emulator-5554", "android", "mobile", emulator = true).isRunningEmulator)
    }

    @Test
    fun theDesktopComesFirstThenPhonesThenBrowsers() {
        val sorted = Device.sorted(listOf(device("edge", "web", "web"), device("b", "android", "mobile"), device("windows", "windows", "desktop"), device("a", "android", "mobile")))
        assertEquals(listOf("windows", "a", "b", "edge"), sorted.map { it.id })
    }

    @Test
    fun simulatedDevicesComeLastNotBeforeThisComputer() {
        // "Android phone" sorts before "Windows" by name: it was listed, and chosen, first.
        val sorted = Device.sorted(
            listOf(
                device("windows", "windows", "desktop", name = "Windows"),
                device("android-phone", "windows", "desktop", name = "Android phone"),
                device("edge", "web", "web"),
            ),
        )
        assertEquals(listOf("windows", "edge", "android-phone"), sorted.map { it.id })
    }

    @Test
    fun theDeviceARunGoesToUnlessOneIsChosen() {
        val host = device("windows", "windows", "desktop", name = "Windows")
        val simulated = device("android-phone", "windows", "desktop", name = "Android phone")
        val stopped = device("Pixel_7", "android", "mobile", emulator = true, name = "Pixel 7")
        val emulator = device("emulator-5554", "android", "mobile", emulator = true, name = "Pixel 7")
        val phone = device("R58M", "android", "mobile", name = "Galaxy A51")
        val edge = device("edge", "web", "web")
        // Never a simulated device, nor an emulator that has to be started: this computer.
        assertEquals("windows", Device.preferred(listOf(simulated, stopped, host, edge))?.id)
        // A running emulator before this computer, a phone plugged in before both.
        assertEquals("emulator-5554", Device.preferred(listOf(simulated, host, emulator))?.id)
        assertEquals("R58M", Device.preferred(listOf(simulated, host, emulator, phone))?.id)
        // A phone that cannot be used (unauthorized) is passed over.
        assertEquals("windows", Device.preferred(listOf(host, phone.copy(available = false)))?.id)
        // Only simulated devices: none, and frus picks.
        assertNull(Device.preferred(listOf(simulated)))
    }

    @Test
    fun emulatorsAndUnsupportedPlatforms() {
        assertEquals(listOf(Emulator("Pixel_7", "Pixel 7", "android")), Emulator.from(listOf(mapOf("id" to "Pixel_7", "name" to "Pixel 7", "platformType" to "android"), "junk")))
        val reasons = unsupportedReasons(
            mapOf(
                "platforms" to listOf("windows"),
                "platformTypes" to mapOf(
                    "windows" to mapOf("isSupported" to true),
                    "android" to mapOf("isSupported" to false, "reasons" to listOf(mapOf("reasonText" to "no [package.metadata.android]", "fixText" to "…"))),
                    "web" to mapOf("isSupported" to false),
                ),
            ),
        )
        assertEquals(mapOf("android" to "no [package.metadata.android]", "web" to "not set up"), reasons)
    }

    @Test
    fun reloadOnSaveModes() {
        assertFalse(ReloadOnSave.NEVER.reloads(manual = true, dirty = true))
        assertTrue(ReloadOnSave.MANUAL.reloads(manual = true, dirty = false))
        assertFalse(ReloadOnSave.MANUAL.reloads(manual = false, dirty = true))
        assertFalse(ReloadOnSave.MANUAL_IF_DIRTY.reloads(manual = true, dirty = false))
        assertTrue(ReloadOnSave.MANUAL_IF_DIRTY.reloads(manual = true, dirty = true))
        assertTrue(ReloadOnSave.ALL.reloads(manual = false, dirty = false))
    }

    @Test
    fun theRunArguments() {
        // Hot reload on by default, said as --hot so that frus 0.5.0, whose default is off, has it.
        assertEquals(HotMode.ON, FrusRunOptions().hot)
        assertEquals(listOf("run", "--machine", "--hot"), runArguments(null, false, HotMode.ON, emptyList()))
        assertEquals(
            listOf("run", "--machine", "-d", "emulator-5554", "--release", "--", "--x", "y z"),
            runArguments("emulator-5554", true, HotMode.ON, listOf("--x", "y z")),
        )
        assertEquals(listOf("run", "--machine", "-d", "web", "--no-hot"), runArguments("web", false, HotMode.OFF, emptyList()))
        // frus 0.5.0 refuses --no-hot; off is its default.
        assertEquals(
            listOf("run", "--machine", "-d", "web"),
            runArguments("web", false, HotMode.OFF, emptyList(), knowsNoHot = false),
        )
        assertEquals(listOf("run", "--machine"), runArguments("  ", false, HotMode.FRUS, emptyList()))
    }

    @Test
    fun hotModesSayWhatTheyDo() {
        // An older plugin saved its default as DEFAULT: it is on now.
        assertEquals(HotMode.ON, HotMode.named("DEFAULT"))
        assertEquals(HotMode.ON, HotMode.named(null))
        assertEquals(HotMode.OFF, HotMode.named("OFF"))
        // No label promises a default the frus installed may not have.
        assertTrue(HotMode.entries.none { it.label.contains("debug build") })
        assertEquals(HotMode.entries.size, HotMode.entries.map { it.hint }.toSet().size)
    }

    @Test
    fun anApplicationDependsOnFrus() {
        assertTrue(isApplicationManifest("[package]\nname = \"a\"\n\n[dependencies]\nfrus = \"0.5\"\n"))
        assertTrue(isApplicationManifest("[package]\nname = \"a\"\n[dependencies]\nfrus.workspace = true\n"))
        assertFalse(isApplicationManifest("[workspace]\nmembers = []\n[dependencies]\nfrus = \"0.5\"\n"))
        assertFalse(isApplicationManifest("[package]\nname = \"a\"\n[dependencies]\nfrus-core = \"0.5\"\n"))
    }

    @Test
    fun applicationsAreFoundNearestFirstSkippingBuildOutput() {
        val root = temp.root
        fun app(path: String) = File(root, path).apply { mkdirs(); File(this, "Cargo.toml").writeText("[package]\nname=\"x\"\n[dependencies]\nfrus = \"0.5\"\n") }
        app("apps/b")
        app("apps/a")
        app("target/debug/x")
        File(root, "lib").mkdirs()
        File(root, "lib/Cargo.toml").writeText("[package]\nname=\"lib\"\n")
        assertEquals(listOf("a", "b"), findApplications(root).map { it.name })
        app("")
        assertEquals(root, findApplications(root).first())
    }

    @Test
    fun aNewProjectsName() {
        assertNotNull(nameProblem("", null))
        assertNotNull(nameProblem("My_app", null))
        assertNotNull(nameProblem("1app", null))
        assertNotNull(nameProblem("my app", null))
        assertNull(nameProblem("my_app-2", null))
        File(temp.root, "taken").mkdirs()
        File(temp.root, "taken/file").writeText("x")
        File(temp.root, "empty").mkdirs()
        assertNotNull(nameProblem("taken", temp.root))
        assertNull(nameProblem("empty", temp.root))
    }

    @Test
    fun anOrganisation() {
        assertNull(orgProblem("com.example"))
        assertNull(orgProblem("dev.frus_rs.apps"))
        assertNotNull(orgProblem("example"))
        assertNotNull(orgProblem("com.1example"))
        assertNotNull(orgProblem("com..example"))
    }

    @Test
    fun createArgumentsLeaveOutTheDefaultOrganisation() {
        val parent = File("parent")
        assertEquals(listOf("create", File(parent, "app").path), createArguments(parent, "app", DEFAULT_ORG))
        assertEquals(listOf("create", File(parent, "app").path, "--org", "dev.acme"), createArguments(parent, "app", "dev.acme"))
    }

    @Test
    fun aCopyOfTheProgramFollowsANewInstall() {
        val program = File(temp.root, "frus.exe").apply { writeText("one") }
        val dir = File(temp.root, "bin")
        val first = FrusProgram.runnableCopy(program, dir)
        assertEquals("one", first.readText())
        assertEquals(first, FrusProgram.runnableCopy(program, dir))
        program.writeText("second")
        program.setLastModified(program.lastModified() + 5000)
        val second = FrusProgram.runnableCopy(program, dir)
        assertEquals("second", second.readText())
        assertFalse(first.exists())
        assertEquals(listOf(second.name), dir.list()!!.toList())
        assertTrue(second.name.endsWith(".exe"))
    }
}

class MachineOutputTest {
    @Test
    fun eventsBecomeConsoleText() {
        assertEquals(Said("hello", Shown.APP), MachineOutput.translate("""{"event":"app.log","params":{"log":"hello"}}"""))
        assertEquals(Said("boom", Shown.APP_ERROR), MachineOutput.translate("""{"event":"app.log","params":{"log":"boom","error":true}}"""))
        assertEquals(Said("Building…", Shown.TOOL), MachineOutput.translate("""{"event":"app.progress","params":{"message":"Building…"}}"""))
        assertEquals(Said("Running on Pixel 7…", Shown.TOOL), MachineOutput.translate("""{"event":"app.start","params":{"deviceName":"Pixel 7"}}"""))
        assertTrue(MachineOutput.translate("""{"event":"app.started","params":{}}""").started)
        assertEquals("Reloaded in 4.9 s.", MachineOutput.translate("""{"event":"app.restarted","params":{"fullRestart":false,"seconds":4.91}}""").text)
        assertEquals("Restarted in 7.0 s.", MachineOutput.translate("""{"event":"app.restarted","params":{"fullRestart":true,"seconds":7}}""").text)
        val stop = MachineOutput.translate("""{"event":"app.stop","params":{"error":"the build failed"}}""")
        assertTrue(stop.stopped)
        assertEquals("the build failed", stop.text)
        assertNull(MachineOutput.translate("""{"event":"app.stop","params":{}}""").text)
        assertNull(MachineOutput.translate("""{"event":"daemon.connected","params":{}}""").text)
        assertEquals(Said("warning: unused", Shown.TOOL), MachineOutput.translate("warning: unused"))
    }

    @Test
    fun everyEndIsShownWithItsExitCodeAndTheLastError() {
        val clean = RunEnding()
        clean.saw(null, Shown.TOOL_ERROR, stopped = true)
        assertEquals(Said("frus run ended (exit code 0).", Shown.TOOL), clean.message(0))
        assertEquals("frus run ended (exit code 0, without saying it was stopping).", RunEnding().message(0).text)

        val failed = RunEnding()
        failed.sawStderr("   Compiling app v0.1.0")
        failed.saw("the engine says hello", Shown.TOOL)
        failed.sawStderr("error: frus run has no option `--no-hot`")
        failed.sawStderr("warning: unused")
        val said = failed.message(1)
        assertEquals(Shown.TOOL_ERROR, said.shown)
        assertEquals("frus run failed (exit code 1). The last error: error: frus run has no option `--no-hot`", said.text)

        val stopped = RunEnding()
        stopped.saw("the hot-reload engine stopped (exit code: 1)", Shown.TOOL_ERROR, stopped = true)
        assertTrue(stopped.message(1).text!!.endsWith("The last error: the hot-reload engine stopped (exit code: 1)"))
        val dxPanic = RunEnding()
        dxPanic.sawStderr("thread 'main' panicked at src/main.rs:3:5:")
        assertTrue(dxPanic.message(101).text!!.contains("panicked"))
        assertTrue(RunEnding().message(-1).text!!.contains("It said no error"))
    }

    @Test
    fun commandsAreLines() {
        assertEquals("""{"id":1,"method":"app.key","params":{"key":"p"}}""" + "\n", MachineOutput.command(1, "app.key", mapOf("key" to "p")))
        assertEquals("""{"id":2,"method":"app.stop"}""" + "\n", MachineOutput.command(2, "app.stop"))
    }

    @Test
    fun linesArriveInPieces() {
        val buffer = LineBuffer()
        assertEquals(emptyList<String>(), buffer.take("{\"a\""))
        assertEquals(listOf("{\"a\":1}", "x"), buffer.take(":1}\r\nx\ny"))
        assertEquals("y", buffer.rest())
        assertNull(buffer.rest())
    }
}
