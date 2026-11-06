// The plugin's actions: the device selector, reload and restart, the keys of `frus run`, the
// emulators, and the tool's own commands.

package dev.frus.intellij

import com.intellij.execution.RunContentExecutor
import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.process.ColoredProcessHandler
import com.intellij.icons.AllIcons
import com.intellij.notification.NotificationAction
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.actionSystem.ActionUpdateThread
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.DataContext
import com.intellij.openapi.actionSystem.DefaultActionGroup
import com.intellij.openapi.actionSystem.Separator
import com.intellij.openapi.actionSystem.ex.ComboBoxAction
import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.progress.Task
import com.intellij.openapi.project.DumbAwareAction
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.Messages
import com.intellij.openapi.ui.popup.JBPopupFactory
import java.io.File
import java.nio.charset.StandardCharsets
import java.util.concurrent.ExecutionException
import java.util.concurrent.TimeUnit
import javax.swing.JComponent

/** Where runs go: the toolbar's device selector. Its menu holds the devices that can run the
 *  application now, then the phones and tablets simulated on this computer, then the emulators
 *  that can be started, then the one that makes a new emulator. */
class DeviceSelectorAction : ComboBoxAction() {
    override fun getActionUpdateThread() = ActionUpdateThread.BGT

    override fun update(e: AnActionEvent) {
        val project = e.project
        val p = e.presentation
        if (project == null) {
            p.isEnabledAndVisible = false
            return
        }
        val daemon = FrusDaemon.get()
        daemon.ensureStarted()
        val state = FrusProject.get(project)
        // Shown in projects with a frus application only: the IDE's own selector serves the rest.
        if (!hasApplication(project)) {
            p.isEnabledAndVisible = false
            return
        }
        p.isEnabledAndVisible = true
        val device = state.device()
        p.icon = FrusIcons.Frus
        p.text = when {
            device != null -> device.name
            daemon.problem != null -> "frus: not running"
            !daemon.isRunning -> "Looking for devices…"
            else -> "No device"
        }
        p.description = daemon.problem ?: "frus: the device the application runs on"
    }

    override fun createPopupActionGroup(button: JComponent, context: DataContext): DefaultActionGroup {
        val group = DefaultActionGroup()
        val project = com.intellij.openapi.actionSystem.CommonDataKeys.PROJECT.getData(context) ?: return group
        val daemon = FrusDaemon.get()
        val unsupported = FrusProject.get(project).applications().firstOrNull()?.let(daemon::unsupported).orEmpty()
        val running = daemon.list().filter { !it.isStoppedEmulator }
        val entry = { d: Device ->
            val why = when {
                !d.available -> d.unavailableReason
                unsupported.containsKey(d.platform) -> "not for this application: ${unsupported[d.platform]}"
                else -> null
            }
            SelectDevice(d, why)
        }
        running.filter { !it.isSimulated }.forEach { group.add(entry(it)) }
        val simulated = running.filter { it.isSimulated }
        if (simulated.isNotEmpty()) {
            group.add(Separator.create("Simulated"))
            simulated.forEach { group.add(entry(it)) }
        }
        group.add(Separator.create("Emulators"))
        val up = daemon.list().filter { it.isRunningEmulator }.map { it.name }.toSet()
        daemon.emulators.filter { it.name !in up && it.name.replace('_', ' ') !in up }.forEach { group.add(StartEmulator(it)) }
        group.add(CreateEmulatorAction())
        if (!daemon.isRunning) {
            group.add(Separator.create())
            group.add(object : DumbAwareAction("Start frus Again", daemon.problem, AllIcons.Actions.Restart) {
                override fun actionPerformed(e: AnActionEvent) = daemon.restart()
            })
        }
        return group
    }

    private class SelectDevice(private val device: Device, why: String?) :
        DumbAwareAction(device.name + (why?.let { "  ($it)" } ?: ""), "${device.id} • ${device.detail}", null) {
        override fun actionPerformed(e: AnActionEvent) {
            e.project?.let { FrusProject.get(it).choose(device.id) }
        }
    }

    private class StartEmulator(private val emulator: Emulator) :
        DumbAwareAction("Start ${emulator.name}", "${emulator.platformType} emulator", AllIcons.Actions.Execute) {
        override fun actionPerformed(e: AnActionEvent) = launchEmulator(e.project, emulator)
    }

    companion object {
        private val known = java.util.concurrent.ConcurrentHashMap<String, Pair<Long, Boolean>>()

        /** Whether the project holds a frus application, looked for at most every 10 seconds. */
        fun hasApplication(project: Project): Boolean {
            val key = project.locationHash
            val now = System.currentTimeMillis()
            known[key]?.let { (at, has) -> if (now - at < 10_000) return has }
            val has = FrusProject.get(project).applications().isNotEmpty()
            known[key] = now to has
            return has
        }
    }
}

/** Starts an emulator; it is chosen once it is up. */
fun launchEmulator(project: Project?, emulator: Emulator) {
    val daemon = FrusDaemon.get()
    if (project != null) {
        daemon.awaitEmulator(emulator.name) { device ->
            if (project.isDisposed) return@awaitEmulator
            FrusProject.get(project).choose(device.id)
            FrusDaemon.notify(NotificationType.INFORMATION, "${device.name} is running and chosen.", project)
        }
    }
    daemon.launchEmulator(emulator.id).whenComplete { _, error ->
        if (error != null) FrusDaemon.notify(NotificationType.ERROR, cause(error), project)
    }
}

private fun cause(error: Throwable): String =
    ((error as? ExecutionException)?.cause ?: (error as? java.util.concurrent.CompletionException)?.cause ?: error).message ?: error.toString()

class LaunchEmulatorAction : DumbAwareAction() {
    override fun getActionUpdateThread() = ActionUpdateThread.BGT

    override fun actionPerformed(e: AnActionEvent) {
        val daemon = FrusDaemon.get()
        daemon.ensureStarted()
        val emulators = daemon.emulators
        if (emulators.isEmpty()) {
            Messages.showInfoMessage(e.project, "There is no emulator yet: create one first (Tools › frus › Create Emulator).", "frus")
            return
        }
        JBPopupFactory.getInstance().createPopupChooserBuilder(emulators)
            .setTitle("Which Emulator Should Start?")
            .setRenderer(textRenderer<Emulator> { "${it.name}  (${it.platformType})" })
            .setItemChosenCallback { launchEmulator(e.project, it) }
            .createPopup()
            .showCenteredInCurrentWindow(e.project ?: return)
    }
}

/** Makes an Android emulator with the name asked for, and offers to start it. */
class CreateEmulatorAction : DumbAwareAction("Create an Android Emulator…", "Installs what it needs", AllIcons.General.Add) {
    override fun getActionUpdateThread() = ActionUpdateThread.BGT

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project
        val name = Messages.showInputDialog(project, "A name for the new Android emulator:", "frus", null, "frus_emulator", null) ?: return
        val daemon = FrusDaemon.get()
        daemon.ensureStarted()
        ProgressManager.getInstance().run(object : Task.Backgroundable(project, "Creating an Android emulator", true) {
            override fun run(indicator: ProgressIndicator) {
                indicator.text = "Creating an Android emulator (the first one may download a system image)…"
                val stop = daemon.onMessage { _, message -> indicator.text2 = message }
                val created = try {
                    val future = daemon.createEmulator(name.trim())
                    while (!future.isDone) {
                        indicator.checkCanceled()
                        Thread.sleep(200)
                    }
                    future.get(1, TimeUnit.SECONDS)
                } catch (error: Throwable) {
                    if (error is com.intellij.openapi.progress.ProcessCanceledException) throw error
                    FrusDaemon.notify(NotificationType.ERROR, "The emulator was not created: ${cause(error)}", project)
                    return
                } finally {
                    stop()
                }
                NotificationGroupManager.getInstance().getNotificationGroup("frus")
                    .createNotification("frus", "$created was created.", NotificationType.INFORMATION)
                    .addAction(NotificationAction.createSimpleExpiring("Start it") {
                        launchEmulator(project, Emulator(created, created.replace('_', ' '), "android"))
                    })
                    .notify(project)
            }
        })
    }
}

/** An action on the frus runs of the project. */
abstract class RunAction : DumbAwareAction() {
    override fun getActionUpdateThread() = ActionUpdateThread.BGT

    override fun update(e: AnActionEvent) {
        e.presentation.isEnabled = e.project?.let { FrusProject.get(it).live().isNotEmpty() } == true
    }

    override fun actionPerformed(e: AnActionEvent) {
        e.project?.let { project -> FrusProject.get(project).live().forEach(::perform) }
    }

    abstract fun perform(run: FrusProcessHandler)
}

class ReloadAction : RunAction() {
    override fun perform(run: FrusProcessHandler) = run.reload()
}

class RestartAction : RunAction() {
    override fun perform(run: FrusProcessHandler) = run.restart()
}

/** A key of `frus run`'s, as its terminal has. */
abstract class KeyAction(private val key: String) : RunAction() {
    override fun perform(run: FrusProcessHandler) = run.key(key)
}

class ToggleDebugPaintAction : KeyAction("p")
class ToggleInspectorAction : KeyAction("i")
class TogglePlatformAction : KeyAction("o")
class ToggleBrightnessAction : KeyAction("b")
class TogglePerformanceOverlayAction : KeyAction("P")
class PrintWidgetTreeAction : KeyAction("w")
class PrintFocusTreeAction : KeyAction("f")
class PrintSemanticsTreeAction : KeyAction("S")
class ScreenshotAction : KeyAction("s")
class DetachAction : KeyAction("d")

/** A `frus` or `cargo` command of its own, in the Run tool window. */
private fun runTool(project: Project, title: String, command: (String) -> GeneralCommandLine) {
    ProgressManager.getInstance().run(object : Task.Backgroundable(project, "Finding frus", true) {
        override fun run(indicator: ProgressIndicator) {
            val frus = try {
                FrusProgram.locate(indicator)
            } catch (e: FrusException) {
                FrusDaemon.notify(NotificationType.ERROR, e.message ?: "frus is not there", project)
                return
            }
            com.intellij.openapi.application.ApplicationManager.getApplication().invokeLater {
                val line = command(frus).withCharset(StandardCharsets.UTF_8)
                    .withParentEnvironmentType(GeneralCommandLine.ParentEnvironmentType.CONSOLE)
                project.basePath?.let { line.withWorkDirectory(it) }
                RunContentExecutor(project, ColoredProcessHandler(line))
                    .withTitle(title)
                    .withActivateToolWindow(true)
                    .run()
            }
        }
    })
}

/** `frus doctor -v`: what this machine has for each platform, and what it lacks. */
class DoctorAction : DumbAwareAction() {
    override fun getActionUpdateThread() = ActionUpdateThread.BGT

    override fun update(e: AnActionEvent) {
        e.presentation.isEnabled = e.project != null
    }

    override fun actionPerformed(e: AnActionEvent) {
        runTool(e.project ?: return, "frus doctor") { GeneralCommandLine(it, "doctor", "-v") }
    }
}

/** `cargo install frus-cli`: the newest `frus` from crates.io. The copy the plugin runs follows
 *  when the install lands. */
class UpdateAction : DumbAwareAction() {
    override fun getActionUpdateThread() = ActionUpdateThread.BGT

    override fun update(e: AnActionEvent) {
        e.presentation.isEnabled = e.project != null
    }

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val configured = FrusSettings.get().path
        if (configured.isNotEmpty()) {
            FrusDaemon.notify(
                NotificationType.INFORMATION,
                "This updates the frus that cargo installs; the settings name $configured, which stays as it is.",
                project,
            )
        }
        val cargo = FrusProgram.resolve("cargo")?.path ?: "cargo"
        val line = GeneralCommandLine(cargo, "install", "frus-cli", "--locked")
            .withCharset(StandardCharsets.UTF_8)
            .withParentEnvironmentType(GeneralCommandLine.ParentEnvironmentType.CONSOLE)
        RunContentExecutor(project, ColoredProcessHandler(line))
            .withTitle("frus update")
            .withActivateToolWindow(true)
            .withAfterCompletion { FrusDaemon.get().restart() }
            .run()
    }
}

internal fun File.isRust(): Boolean = extension == "rs"
