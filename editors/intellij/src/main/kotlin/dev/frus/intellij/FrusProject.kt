// What a project remembers (the device chosen) and the frus runs it has going.

package dev.frus.intellij

import com.intellij.openapi.Disposable
import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage
import com.intellij.openapi.components.StoragePathMacros
import com.intellij.openapi.project.Project
import com.intellij.util.Alarm
import java.io.File
import java.util.concurrent.CopyOnWriteArrayList

@Service(Service.Level.PROJECT)
@State(name = "FrusProject", storages = [Storage(StoragePathMacros.WORKSPACE_FILE)])
class FrusProject(private val project: Project) : PersistentStateComponent<FrusProject.Data>, Disposable {
    class Data {
        /** The device runs go to unless their configuration names one. */
        var device: String? = null
    }

    private var data = Data()
    private val runs = CopyOnWriteArrayList<FrusProcessHandler>()
    private val reloadAlarm = Alarm(Alarm.ThreadToUse.POOLED_THREAD, this)

    override fun getState(): Data = data

    override fun loadState(state: Data) {
        data = state
    }

    /** The device chosen in the toolbar, if it is still there; else the preferred one: a phone
     *  plugged in, a running emulator, this computer (never a simulated device unless chosen). */
    fun device(): Device? {
        val daemon = FrusDaemon.get()
        return data.device?.let(daemon::get) ?: Device.preferred(daemon.list())
    }

    /** The id chosen, whether or not the device is there now. */
    val chosenId: String? get() = data.device

    fun choose(id: String) {
        data.device = id
        com.intellij.ide.ActivityTracker.getInstance().inc()
    }

    /** The frus applications of the project. */
    fun applications(): List<File> = project.basePath?.let { findApplications(File(it)) }.orEmpty()

    /** The runs going now. */
    fun live(): List<FrusProcessHandler> = runs.filter { !it.isProcessTerminated && !it.isProcessTerminating }

    fun started(run: FrusProcessHandler) {
        runs += run
        run.addProcessListener(object : com.intellij.execution.process.ProcessListener {
            override fun processTerminated(event: com.intellij.execution.process.ProcessEvent) {
                runs -= run
                com.intellij.ide.ActivityTracker.getInstance().inc()
            }
        })
        com.intellij.ide.ActivityTracker.getInstance().inc()
    }

    /** Reloads every run, soon: saves that come together (auto save writes every open file) make
     *  one reload. */
    fun reloadSoon() {
        if (live().isEmpty()) return
        reloadAlarm.cancelAllRequests()
        reloadAlarm.addRequest({ live().forEach { it.reload() } }, 150)
    }

    override fun dispose() {}

    companion object {
        fun get(project: Project): FrusProject = project.getService(FrusProject::class.java)
    }
}
