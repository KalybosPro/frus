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
        assertEquals(listOf("run", "--machine"), runArguments(null, false, HotMode.DEFAULT, emptyList()))
        assertEquals(
            listOf("run", "--machine", "-d", "emulator-5554", "--release", "--hot", "--", "--x", "y z"),
            runArguments("emulator-5554", true, HotMode.ON, listOf("--x", "y z")),
        )
        assertEquals(listOf("run", "--machine", "-d", "web", "--no-hot"), runArguments("web", false, HotMode.OFF, emptyList()))
        assertEquals(listOf("run", "--machine"), runArguments("  ", false, HotMode.DEFAULT, emptyList()))
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
