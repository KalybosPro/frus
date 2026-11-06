// The plugin's settings (Settings › Tools › frus), the same for every project.

package dev.frus.intellij

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage
import com.intellij.openapi.fileChooser.FileChooserDescriptorFactory
import com.intellij.openapi.options.Configurable
import com.intellij.openapi.ui.ComboBox
import com.intellij.openapi.ui.TextFieldWithBrowseButton
import com.intellij.ui.ColoredListCellRenderer
import com.intellij.util.ui.FormBuilder
import javax.swing.JComponent
import javax.swing.JList

@Service(Service.Level.APP)
@State(name = "FrusSettings", storages = [Storage("frus.xml")])
class FrusSettings : PersistentStateComponent<FrusSettings.Data> {
    class Data {
        /** The `frus` program to use; empty: the one on the PATH, installed when there is none. */
        var path: String = ""
        var reloadOnSave: String = ReloadOnSave.MANUAL.name

        /** The organisation the last project was created with, offered for the next one. */
        var organization: String = DEFAULT_ORG
    }

    private var data = Data()

    override fun getState(): Data = data

    override fun loadState(state: Data) {
        data = state
    }

    var path: String
        get() = data.path.trim()
        set(value) {
            data.path = value.trim()
        }

    var reloadOnSave: ReloadOnSave
        get() = ReloadOnSave.entries.find { it.name == data.reloadOnSave } ?: ReloadOnSave.MANUAL
        set(value) {
            data.reloadOnSave = value.name
        }

    var organization: String
        get() = data.organization
        set(value) {
            data.organization = value
        }

    companion object {
        fun get(): FrusSettings = ApplicationManager.getApplication().getService(FrusSettings::class.java)
    }
}

class FrusConfigurable : Configurable {
    private val path = TextFieldWithBrowseButton()
    private val reload = ComboBox(ReloadOnSave.entries.toTypedArray())

    override fun getDisplayName(): String = "frus"

    override fun createComponent(): JComponent {
        path.addBrowseFolderListener(
            null,
            FileChooserDescriptorFactory.singleFile().withTitle("The frus Program"),
        )
        reload.renderer = textRenderer { it.label }
        return FormBuilder.createFormBuilder()
            .addLabeledComponent("frus program:", path)
            .addTooltip("Empty: the frus on the PATH, installed with cargo install frus-cli when there is none.")
            .addLabeledComponent("Reload on save:", reload)
            .addTooltip("Reloading keeps every state with hot reload, and the state of the components that keep one otherwise.")
            .addComponentFillVertically(javax.swing.JPanel(), 0)
            .panel
    }

    override fun isModified(): Boolean {
        val settings = FrusSettings.get()
        return path.text.trim() != settings.path || reload.item != settings.reloadOnSave
    }

    override fun apply() {
        val settings = FrusSettings.get()
        val changed = path.text.trim() != settings.path
        settings.path = path.text
        settings.reloadOnSave = reload.item
        if (changed) FrusProgram.forget()
    }

    override fun reset() {
        val settings = FrusSettings.get()
        path.text = settings.path
        reload.item = settings.reloadOnSave
    }
}

/** A list cell that shows `text` of its value, and nothing for none. */
internal fun <T> textRenderer(text: (T) -> String) = object : ColoredListCellRenderer<T>() {
    override fun customizeCellRenderer(list: JList<out T>, value: T?, index: Int, selected: Boolean, hasFocus: Boolean) {
        if (value != null) append(text(value))
    }
}
