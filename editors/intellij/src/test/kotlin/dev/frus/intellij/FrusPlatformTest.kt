package dev.frus.intellij

import com.intellij.execution.RunManager
import com.intellij.execution.configurations.ConfigurationTypeUtil
import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.configurations.RuntimeConfigurationError
import com.intellij.execution.process.ProcessEvent
import com.intellij.execution.process.ProcessListener
import com.intellij.execution.process.ProcessOutputType
import com.intellij.execution.process.ProcessOutputTypes
import com.intellij.openapi.util.Key
import com.intellij.openapi.util.JDOMUtil
import com.intellij.testFramework.fixtures.BasePlatformTestCase
import java.io.File
import java.nio.charset.StandardCharsets
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.TimeUnit

class FrusPlatformTest : BasePlatformTestCase() {
    private fun java(): String = File(System.getProperty("java.home"), "bin/java").path

    /** The jar or folder a class is loaded from. Not its code source: the IDE's own loader,
     *  which brings the Kotlin library, gives none. */
    private fun classPathEntry(type: Class<*>): String {
        val name = type.name.replace('.', '/') + ".class"
        val url = type.classLoader.getResource(name) ?: error("no $name")
        return when (url.protocol) {
            "jar" -> File(java.net.URI(url.path.substringBefore("!/"))).path
            else -> File(url.toURI()).path.removeSuffix(File(name).path).trimEnd(File.separatorChar)
        }
    }

    /** The fake `frus run --machine`, its output as the console receives it. */
    private fun fakeRun(vararg args: String): Pair<FrusProcessHandler, MutableList<Pair<String, Key<*>>>> {
        // Only what the fake needs: the whole test class path is too long a command line for
        // Windows (CreateProcess error 206).
        val classPath = listOf(FakeFrus::class.java, Json::class.java, Unit::class.java)
            .map(::classPathEntry)
            .distinct()
            .joinToString(File.pathSeparator)
        val line = GeneralCommandLine(listOf(java(), "-cp", classPath, FakeFrus::class.java.name) + args)
            .withCharset(StandardCharsets.UTF_8)
        val handler = FrusProcessHandler(line)
        val seen = CopyOnWriteArrayList<Pair<String, Key<*>>>()
        handler.addProcessListener(object : ProcessListener {
            override fun onTextAvailable(event: ProcessEvent, outputType: Key<*>) {
                seen += event.text to outputType
            }
        })
        return handler to seen
    }

    private fun waitFor(what: String, seen: List<Pair<String, Key<*>>>, timeout: Long = 30_000) {
        val until = System.currentTimeMillis() + timeout
        while (seen.none { it.first.contains(what) }) {
            if (System.currentTimeMillis() > until) fail("never saw \"$what\" in:\n" + seen.joinToString("") { it.first })
            Thread.sleep(20)
        }
    }

    fun testTheConsoleShowsTheApplicationAndTheToolApart() {
        val (handler, seen) = fakeRun()
        handler.startNotify()
        waitFor("Saving a Rust file reloads it", seen)
        assertTrue(handler.started)
        // Standard error is read on a thread of its own: the build's line can come after
        // the application has started.
        fun typeOf(text: String): Key<*> {
            waitFor(text, seen)
            return seen.first { it.first.contains(text) }.second
        }
        assertEquals(ProcessOutputTypes.STDOUT, typeOf("hello from the app"))
        assertEquals(ProcessOutputTypes.STDERR, typeOf("an error"))
        assertEquals(ProcessOutputType.SYSTEM, typeOf("Running on Fake device…"))
        assertEquals(ProcessOutputType.SYSTEM, typeOf("Compiling app"))
        // Nothing raw: every JSON line was translated.
        assertTrue(seen.none { it.first.contains("\"event\"") })

        handler.reload()
        waitFor("""got app.restart {"fullRestart":false}""", seen)
        waitFor("Reloaded in 1.3 s.", seen)
        handler.restart()
        waitFor("""got app.restart {"fullRestart":true}""", seen)
        handler.key("p")
        waitFor("""got app.key {"key":"p"}""", seen)
        handler.key("too long")

        // Stop asks frus to stop rather than killing it. Not timed: a busy machine can take
        // seconds to end a JVM. The exit code tells: killed (after the handler's 5 s of
        // grace), it would not be 0.
        handler.destroyProcess()
        waitFor("got app.stop", seen)
        assertTrue("it ends", handler.waitFor(TimeUnit.SECONDS.toMillis(30)))
        assertEquals("it ended on app.stop, without being killed", 0, handler.exitCode)
        waitFor("frus run ended (exit code 0).", seen)
    }

    fun testARunThatFailsSaysSoWithItsExitCodeAndTheLastError() {
        val (handler, seen) = fakeRun("fail")
        handler.startNotify()
        assertTrue("it ends", handler.waitFor(TimeUnit.SECONDS.toMillis(30)))
        waitFor("frus run failed (exit code 3). The last error: error: the hot-reload engine stopped", seen)
        assertEquals(ProcessOutputTypes.STDERR, seen.first { it.first.contains("frus run failed") }.second)
    }

    fun testTheHotReloadHintFollowsTheChoice() {
        val editor = FrusRunEditor(project)
        try {
            editor.component // Builds the form.
            val type = ConfigurationTypeUtil.findConfigurationType(FrusConfigurationType::class.java)
            val config = type.factory.createTemplateConfiguration(project) as FrusRunConfiguration
            editor.resetFrom(config)
            assertTrue(editor.hotHintText.contains(HotMode.ON.hint))
            editor.chooseHot(HotMode.OFF)
            assertTrue(editor.hotHintText.contains(HotMode.OFF.hint))
            editor.chooseHot(HotMode.FRUS)
            assertTrue(editor.hotHintText.contains(HotMode.FRUS.hint))
            editor.applyTo(config)
            assertEquals(HotMode.FRUS, config.options.hot)
        } finally {
            com.intellij.openapi.util.Disposer.dispose(editor)
        }
    }

    fun testARunConfigurationIsMadeForTheApplicationAndKeepsItsOptions() {
        val base = File(project.basePath!!).apply { mkdirs() }
        File(base, "Cargo.toml").writeText("[package]\nname = \"app\"\n[dependencies]\nfrus = \"0.5\"\n")
        try {
            ensureConfigurations(project, FrusProject.get(project).applications())
            val type = ConfigurationTypeUtil.findConfigurationType(FrusConfigurationType::class.java)
            val configs = RunManager.getInstance(project).getConfigurationsList(type)
            assertEquals(1, configs.size)
            val config = configs[0] as FrusRunConfiguration
            assertEquals(base, config.applicationDirectory())
            config.checkConfiguration()
            // Not a second one when the project opens again.
            ensureConfigurations(project, FrusProject.get(project).applications())
            assertEquals(1, RunManager.getInstance(project).getConfigurationsList(type).size)

            config.options.device = "emulator-5554"
            config.options.release = true
            config.options.hot = HotMode.OFF
            config.options.arguments = "--a \"b c\""
            val element = org.jdom.Element("configuration")
            config.writeExternal(element)
            val copy = type.factory.createTemplateConfiguration(project) as FrusRunConfiguration
            copy.readExternal(JDOMUtil.load(JDOMUtil.writeElement(element)))
            assertEquals("emulator-5554", copy.options.device)
            assertTrue(copy.options.release)
            assertEquals(HotMode.OFF, copy.options.hot)
            assertEquals("--a \"b c\"", copy.options.arguments)

            copy.options.workingDirectory = File(base, "nowhere").path
            assertThrows(RuntimeConfigurationError::class.java) { copy.checkConfiguration() }
        } finally {
            File(base, "Cargo.toml").delete()
        }
    }
}
