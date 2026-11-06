// A new frus application: where, what it is called, and the organisation its Android package
// starts with; then `frus create`, and the new project opened.

package dev.frus.intellij

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.util.ExecUtil
import com.intellij.ide.impl.OpenProjectTask
import com.intellij.ide.impl.ProjectUtil
import com.intellij.openapi.actionSystem.ActionUpdateThread
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.fileChooser.FileChooserDescriptorFactory
import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.progress.Task
import com.intellij.openapi.project.DumbAwareAction
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.DialogWrapper
import com.intellij.openapi.ui.Messages
import com.intellij.openapi.ui.TextFieldWithBrowseButton
import com.intellij.openapi.ui.ValidationInfo
import com.intellij.ui.components.JBTextField
import com.intellij.util.ui.FormBuilder
import java.io.File
import java.nio.charset.StandardCharsets
import javax.swing.JComponent

class NewProjectAction : DumbAwareAction() {
    override fun getActionUpdateThread() = ActionUpdateThread.BGT

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project
        val dialog = NewProjectDialog(project)
        if (!dialog.showAndGet()) return
        val parent = File(dialog.parent)
        val name = dialog.name
        val org = dialog.org
        FrusSettings.get().organization = org
        ProgressManager.getInstance().run(object : Task.Modal(project, "Creating $name", false) {
            var failure: String? = null

            override fun run(indicator: ProgressIndicator) {
                indicator.isIndeterminate = true
                try {
                    val frus = FrusProgram.locate(indicator)
                    indicator.text = "frus create $name…"
                    parent.mkdirs()
                    val out = ExecUtil.execAndGetOutput(
                        GeneralCommandLine(listOf(frus) + createArguments(parent, name, org))
                            .withCharset(StandardCharsets.UTF_8)
                            .withParentEnvironmentType(GeneralCommandLine.ParentEnvironmentType.CONSOLE),
                    )
                    if (out.exitCode != 0) failure = "frus create failed:\n" + (out.stderr + out.stdout).trim().ifEmpty { "exit code ${out.exitCode}" }
                } catch (error: FrusException) {
                    failure = error.message
                }
            }

            override fun onFinished() {
                failure?.let {
                    Messages.showErrorDialog(project, it, "frus")
                    return
                }
                ApplicationManager.getApplication().invokeLater {
                    ProjectUtil.openOrImport(File(parent, name).toPath(), OpenProjectTask.build().withForceOpenInNewFrame(project != null))
                }
            }
        })
    }
}

class NewProjectDialog(project: Project?) : DialogWrapper(project) {
    private val parentField = TextFieldWithBrowseButton()
    private val nameField = JBTextField("my_app")
    private val orgField = JBTextField(FrusSettings.get().organization)

    val parent: String get() = parentField.text.trim()
    val name: String get() = nameField.text.trim()
    val org: String get() = orgField.text.trim()

    init {
        title = "New frus Project"
        val open = project?.basePath?.let { File(it).parentFile }
        parentField.text = (open ?: File(System.getProperty("user.home"))).path
        parentField.addBrowseFolderListener(
            project,
            FileChooserDescriptorFactory.createSingleFolderDescriptor().withTitle("The Folder the New Project Goes In"),
        )
        init()
    }

    override fun createCenterPanel(): JComponent = FormBuilder.createFormBuilder()
        .addLabeledComponent("Location:", parentField)
        .addLabeledComponent("Name:", nameField)
        .addTooltip("The project's folder, and its package: lowercase letters, digits, _ and -.")
        .addLabeledComponent("Organisation:", orgField)
        .addTooltip("The reverse domain the Android package starts with: <organisation>.<name>.")
        .panel

    override fun getPreferredFocusedComponent(): JComponent = nameField

    override fun doValidateAll(): List<ValidationInfo> = listOfNotNull(
        if (parent.isEmpty()) ValidationInfo("Where should the project go?", parentField) else null,
        nameProblem(name, parent.takeIf { it.isNotEmpty() }?.let(::File))?.let { ValidationInfo(it, nameField) },
        orgProblem(org)?.let { ValidationInfo(it, orgField) },
    )
}
