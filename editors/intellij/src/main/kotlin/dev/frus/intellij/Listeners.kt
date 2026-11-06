// Saving a Rust file reloads the running application, and opening a project with a frus
// application in it makes the run configuration that runs it.

package dev.frus.intellij

import com.intellij.execution.RunManager
import com.intellij.openapi.actionSystem.ActionManager
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.actionSystem.ex.AnActionListener
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.editor.Document
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.fileEditor.FileDocumentManagerListener
import com.intellij.openapi.project.Project
import com.intellij.openapi.project.ProjectLocator
import com.intellij.openapi.startup.ProjectActivity
import java.io.File

/** What the save going on is: by hand (Ctrl+S, the Save All action) or not, and whether it wrote
 *  a Rust file. Saves happen on the UI thread, one at a time. */
private object Saving {
    var manual = false
    var wroteRust = false
}

private val SAVE_ACTIONS = setOf("SaveAll", "SaveDocument")

/** Ctrl+S: a save by hand. Reloads once it is done, as the setting says — even when there was
 *  nothing to write ("manual"), or only when a Rust file was written ("manualIfDirty"). */
class SaveActionListener : AnActionListener {
    override fun beforeActionPerformed(action: AnAction, event: AnActionEvent) {
        if (ActionManager.getInstance().getId(action) !in SAVE_ACTIONS) return
        Saving.manual = true
        Saving.wroteRust = false
    }

    override fun afterActionPerformed(action: AnAction, event: AnActionEvent, result: com.intellij.openapi.actionSystem.AnActionResult) {
        if (ActionManager.getInstance().getId(action) !in SAVE_ACTIONS) return
        val wrote = Saving.wroteRust
        Saving.manual = false
        Saving.wroteRust = false
        val project = event.project ?: return
        val editing = event.getData(CommonDataKeys.VIRTUAL_FILE)?.extension == "rs"
        if (!wrote && !editing) return
        if (FrusSettings.get().reloadOnSave.reloads(manual = true, dirty = wrote)) FrusProject.get(project).reloadSoon()
    }
}

/** A save that is not by hand (auto save, a build, switching away from the IDE) reloads only
 *  with "every save"; one by hand is noted for [SaveActionListener]. */
class SaveListener : FileDocumentManagerListener {
    override fun beforeDocumentSaving(document: Document) {
        val file = FileDocumentManager.getInstance().getFile(document) ?: return
        if (file.extension != "rs") return
        if (Saving.manual) {
            Saving.wroteRust = true
            return
        }
        if (!FrusSettings.get().reloadOnSave.reloads(manual = false, dirty = true)) return
        // Reloaded once written: after this save, which runs now on the UI thread.
        ApplicationManager.getApplication().invokeLater {
            val project = ProjectLocator.getInstance().guessProjectForFile(file) ?: return@invokeLater
            FrusProject.get(project).reloadSoon()
        }
    }
}

/** A project with frus applications: the daemon starts (the toolbar lists the devices), and each
 *  application gets a run configuration if none runs it, so that Run works at once. */
class FrusStartup : ProjectActivity {
    override suspend fun execute(project: Project) {
        val apps = FrusProject.get(project).applications()
        if (apps.isEmpty()) return
        FrusDaemon.get().ensureStarted()
        ApplicationManager.getApplication().invokeLater {
            if (!project.isDisposed) ensureConfigurations(project, apps)
        }
    }
}

internal fun ensureConfigurations(project: Project, apps: List<File>) {
    val manager = RunManager.getInstance(project)
    val type = com.intellij.execution.configurations.ConfigurationTypeUtil.findConfigurationType(FrusConfigurationType::class.java)
    val existing = manager.getConfigurationsList(type).filterIsInstance<FrusRunConfiguration>()
    if (existing.isNotEmpty()) return
    var first: com.intellij.execution.RunnerAndConfigurationSettings? = null
    for (app in apps.take(10)) {
        val settings = manager.createConfiguration(if (apps.size == 1) "frus" else "frus: ${app.name}", type.factory)
        val config = settings.configuration as FrusRunConfiguration
        if (apps.size > 1) config.options.workingDirectory = app.path
        manager.addConfiguration(settings)
        if (first == null) first = settings
    }
    if (manager.selectedConfiguration == null) first?.let { manager.selectedConfiguration = it }
}
