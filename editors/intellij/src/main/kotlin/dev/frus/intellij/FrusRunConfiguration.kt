// The frus run configuration: Run builds the application for the device chosen in the toolbar,
// starts it there, and streams its log into the Run console.

package dev.frus.intellij

import com.intellij.execution.DefaultExecutionResult
import com.intellij.execution.ExecutionException
import com.intellij.execution.ExecutionResult
import com.intellij.execution.Executor
import com.intellij.execution.configurations.ConfigurationFactory
import com.intellij.execution.configurations.ConfigurationTypeBase
import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.configurations.LocatableConfigurationBase
import com.intellij.execution.configurations.LocatableRunConfigurationOptions
import com.intellij.execution.configurations.RunConfiguration
import com.intellij.execution.configurations.RunProfileState
import com.intellij.execution.configurations.RuntimeConfigurationError
import com.intellij.execution.filters.TextConsoleBuilderFactory
import com.intellij.execution.runners.ExecutionEnvironment
import com.intellij.execution.runners.ProgramRunner
import com.intellij.icons.AllIcons
import com.intellij.openapi.actionSystem.ActionManager
import com.intellij.openapi.fileChooser.FileChooserDescriptorFactory
import com.intellij.openapi.options.SettingsEditor
import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.ComboBox
import com.intellij.openapi.ui.TextFieldWithBrowseButton
import com.intellij.openapi.util.IconLoader
import com.intellij.ui.components.JBCheckBox
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBTextField
import com.intellij.util.execution.ParametersListUtil
import com.intellij.util.ui.FormBuilder
import com.intellij.util.ui.UIUtil
import java.io.File
import java.nio.charset.StandardCharsets
import javax.swing.JComponent

object FrusIcons {
    @JvmField val Frus = IconLoader.getIcon("/icons/frus.svg", FrusIcons::class.java)
}

class FrusConfigurationType : ConfigurationTypeBase(
    ID,
    "frus",
    "Run a frus application on a device: the desktop, a phone, an emulator, a browser",
    FrusIcons.Frus,
) {
    init {
        addFactory(FrusConfigurationFactory(this))
    }

    val factory: ConfigurationFactory get() = configurationFactories[0]

    companion object {
        const val ID = "frus"
    }
}

class FrusConfigurationFactory(type: FrusConfigurationType) : ConfigurationFactory(type) {
    override fun getId(): String = FrusConfigurationType.ID

    override fun createTemplateConfiguration(project: Project): RunConfiguration =
        FrusRunConfiguration(project, this, "frus")

    override fun getOptionsClass(): Class<FrusRunOptions> = FrusRunOptions::class.java
}

class FrusRunOptions : LocatableRunConfigurationOptions() {
    var workingDirectory by string("")
    var device by string("")
    var release by property(false)
    var hotMode by string(HotMode.ON.name)
    var arguments by string("")

    var hot: HotMode
        get() = HotMode.named(hotMode)
        set(value) {
            hotMode = value.name
        }
}

class FrusRunConfiguration(project: Project, factory: ConfigurationFactory, name: String) :
    LocatableConfigurationBase<FrusRunOptions>(project, factory, name) {

    public override fun getOptions(): FrusRunOptions = super.getOptions() as FrusRunOptions

    /** The application's directory: the one set, else the project's only (or first) application. */
    fun applicationDirectory(): File? {
        val set = options.workingDirectory.orEmpty().trim()
        if (set.isNotEmpty()) return File(set)
        return FrusProject.get(project).applications().firstOrNull()
    }

    override fun checkConfiguration() {
        val dir = applicationDirectory() ?: throw RuntimeConfigurationError("There is no frus application in this project: set its directory.")
        if (!File(dir, "Cargo.toml").isFile) throw RuntimeConfigurationError("$dir has no Cargo.toml.")
        if (!isApplication(dir)) throw RuntimeConfigurationError("$dir is not a frus application: its Cargo.toml does not depend on frus.")
    }

    override fun getConfigurationEditor(): SettingsEditor<out RunConfiguration> = FrusRunEditor(project)

    override fun getState(executor: Executor, environment: ExecutionEnvironment): RunProfileState = FrusRunState(this)

    override fun suggestedName(): String? = applicationDirectory()?.let { "frus: ${it.name}" }
}

class FrusRunState(private val configuration: FrusRunConfiguration) : RunProfileState {
    override fun execute(executor: Executor, runner: ProgramRunner<*>): ExecutionResult {
        val project = configuration.project
        val options = configuration.options
        val dir = configuration.applicationDirectory() ?: throw ExecutionException("There is no frus application in this project.")
        val frus = FrusProgram.readyOrNull() ?: try {
            ProgressManager.getInstance().runProcessWithProgressSynchronously<String, FrusException>(
                { FrusProgram.locate(ProgressManager.getInstance().progressIndicator) },
                "Finding frus",
                true,
                project,
            )
        } catch (e: FrusException) {
            throw ExecutionException(e.message)
        }
        val device = options.device.orEmpty().trim().ifEmpty { FrusProject.get(project).let { it.chosenId ?: it.device()?.id }.orEmpty() }
        val args = runArguments(
            device,
            options.release,
            options.hot,
            ParametersListUtil.parse(options.arguments.orEmpty()),
            knowsNoHot = FrusProgram.knowsNoHot,
        )
        val commandLine = GeneralCommandLine(listOf(frus) + args)
            .withWorkDirectory(dir)
            .withCharset(StandardCharsets.UTF_8)
            .withParentEnvironmentType(GeneralCommandLine.ParentEnvironmentType.CONSOLE)
        val handler = FrusProcessHandler(commandLine)
        val console = TextConsoleBuilderFactory.getInstance().createBuilder(project).console
        console.attachToProcess(handler)
        handler.show("frus ${args.joinToString(" ")}", Shown.TOOL)
        FrusProject.get(project).started(handler)
        handler.startNotify()
        val actions = ActionManager.getInstance()
        return DefaultExecutionResult(console, handler, *listOfNotNull(
            actions.getAction("frus.Reload"),
            actions.getAction("frus.Restart"),
        ).toTypedArray())
    }
}

class FrusRunEditor(private val project: Project) : SettingsEditor<FrusRunConfiguration>() {
    private val directory = TextFieldWithBrowseButton()
    private val device = JBTextField()
    private val release = JBCheckBox("Release build")
    private val hot = ComboBox(HotMode.entries.toTypedArray())
    private val hotHint = JBLabel().apply {
        componentStyle = UIUtil.ComponentStyle.SMALL
        fontColor = UIUtil.FontColor.BRIGHTER
    }
    private val arguments = JBTextField()

    override fun createEditor(): JComponent {
        directory.addBrowseFolderListener(
            project,
            FileChooserDescriptorFactory.createSingleFolderDescriptor().withTitle("The Application's Directory"),
        )
        device.emptyText.text = "The device chosen in the toolbar"
        (directory.textField as? JBTextField)?.emptyText?.text = "The project's frus application"
        hot.renderer = textRenderer { it.label }
        // The hint is the chosen mode's, and follows the choice.
        hot.addItemListener { showHotHint() }
        showHotHint()
        return FormBuilder.createFormBuilder()
            .addLabeledComponent("Application directory:", directory)
            .addTooltip("Where its Cargo.toml is.")
            .addLabeledComponent("Device:", device)
            .addTooltip("A device id as frus devices prints it, or android, web, desktop.")
            .addComponent(release)
            .addLabeledComponent("Hot reload:", hot)
            .addComponentToRightColumn(hotHint, 0)
            .addLabeledComponent("Application arguments:", arguments)
            .addTooltip("Passed to the application (desktop).")
            .panel
    }

    override fun resetEditorFrom(configuration: FrusRunConfiguration) {
        val o = configuration.options
        directory.text = o.workingDirectory.orEmpty()
        device.text = o.device.orEmpty()
        release.isSelected = o.release
        hot.item = o.hot
        showHotHint()
        arguments.text = o.arguments.orEmpty()
    }

    /** The text under the hot reload box: what the mode chosen does. */
    internal val hotHintText: String get() = hotHint.text

    internal fun chooseHot(mode: HotMode) {
        hot.item = mode
    }

    private fun showHotHint() {
        hotHint.text = "<html>${(hot.item ?: HotMode.ON).hint}</html>"
    }

    override fun applyEditorTo(configuration: FrusRunConfiguration) {
        val o = configuration.options
        o.workingDirectory = directory.text.trim()
        o.device = device.text.trim()
        o.release = release.isSelected
        o.hot = hot.item
        o.arguments = arguments.text.trim()
    }
}

