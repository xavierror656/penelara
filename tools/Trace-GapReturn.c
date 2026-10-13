/* Trace MotHor's single-call teacher-gap return in an isolated replay.
 * Build as x86. The tracer restores its one-byte breakpoint before detach. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>

#define RETURN_ADDRESS ((LPVOID)0x00458D94)

static int read_u32(HANDLE process, DWORD address, DWORD *value) {
    SIZE_T count = 0;
    return ReadProcessMemory(process, (LPCVOID)(UINT_PTR)address, value, 4, &count)
        && count == 4;
}

static int write_byte(HANDLE process, BYTE value) {
    DWORD old_protect;
    SIZE_T count = 0;
    if (!VirtualProtectEx(process, RETURN_ADDRESS, 1, PAGE_EXECUTE_READWRITE,
                          &old_protect)) return 0;
    int ok = WriteProcessMemory(process, RETURN_ADDRESS, &value, 1, &count)
        && count == 1;
    FlushInstructionCache(process, RETURN_ADDRESS, 1);
    DWORD unused;
    VirtualProtectEx(process, RETURN_ADDRESS, 1, old_protect, &unused);
    return ok;
}

int main(int argc, char **argv) {
    if ((argc != 4 && argc != 5) || sizeof(void *) != 4) {
        fprintf(stderr, "uso: Trace-GapReturn.exe <MotHor PID> <segundos> <max-eventos> [pausa-ms] (x86)\n");
        return 2;
    }
    DWORD pid = (DWORD)strtoul(argv[1], NULL, 10);
    DWORD duration_ms = (DWORD)strtoul(argv[2], NULL, 10) * 1000;
    unsigned max_events = (unsigned)strtoul(argv[3], NULL, 10);
    DWORD pause_ms = argc == 5 ? (DWORD)strtoul(argv[4], NULL, 10) : 0;
    HANDLE process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ
        | PROCESS_VM_WRITE | PROCESS_VM_OPERATION, FALSE, pid);
    if (!process) {
        fprintf(stderr, "OpenProcess: %lu\n", GetLastError());
        return 1;
    }
    BYTE original = 0;
    SIZE_T count = 0;
    if (!ReadProcessMemory(process, RETURN_ADDRESS, &original, 1, &count)
        || count != 1 || original != 0x8B) {
        fprintf(stderr, "byte de retorno inesperado: %02X, error %lu\n",
                original, GetLastError());
        CloseHandle(process);
        return 1;
    }
    if (!DebugActiveProcess(pid)) {
        fprintf(stderr, "DebugActiveProcess: %lu\n", GetLastError());
        CloseHandle(process);
        return 1;
    }
    DebugSetProcessKillOnExit(FALSE);
    int armed = write_byte(process, 0xCC);
    if (!armed) fprintf(stderr, "WriteProcessMemory: %lu\n", GetLastError());
    DWORD stepping_thread = 0;
    unsigned events = 0;
    ULONGLONG started = GetTickCount64();
    printf("elapsed_ms,thread,eax,row,context,stack30,stack34\n");
    fflush(stdout);
    int done = 0;
    while (armed && !done && (stepping_thread
           || (GetTickCount64() - started < duration_ms && events < max_events))) {
        DEBUG_EVENT event;
        if (!WaitForDebugEvent(&event, 500)) {
            if (GetLastError() != ERROR_SEM_TIMEOUT)
                fprintf(stderr, "WaitForDebugEvent: %lu\n", GetLastError());
            continue;
        }
        DWORD disposition = DBG_CONTINUE;
        int process_exited = event.dwDebugEventCode == EXIT_PROCESS_DEBUG_EVENT;
        if (event.dwDebugEventCode == EXCEPTION_DEBUG_EVENT) {
            DWORD code = event.u.Exception.ExceptionRecord.ExceptionCode;
            DWORD address = (DWORD)(UINT_PTR)event.u.Exception.ExceptionRecord.ExceptionAddress;
            if ((code == EXCEPTION_BREAKPOINT && address == (DWORD)(UINT_PTR)RETURN_ADDRESS)
                || (code == EXCEPTION_SINGLE_STEP && event.dwThreadId == stepping_thread)) {
                HANDLE thread = OpenThread(THREAD_GET_CONTEXT | THREAD_SET_CONTEXT,
                                           FALSE, event.dwThreadId);
                CONTEXT context;
                ZeroMemory(&context, sizeof(context));
                context.ContextFlags = CONTEXT_CONTROL | CONTEXT_INTEGER;
                if (!thread || !GetThreadContext(thread, &context)) {
                    fprintf(stderr, "GetThreadContext: %lu\n", GetLastError());
                    disposition = DBG_EXCEPTION_NOT_HANDLED;
                } else if (code == EXCEPTION_BREAKPOINT) {
                    DWORD row = 0, owner = 0, stack30 = 0, stack34 = 0;
                    read_u32(process, context.Esp + 0x6C, &row);
                    read_u32(process, context.Esp + 0x14, &owner);
                    read_u32(process, context.Esp + 0x30, &stack30);
                    read_u32(process, context.Esp + 0x34, &stack34);
                    printf("%llu,%lu,%ld,%lu,%08lX,%ld,%ld\n",
                           GetTickCount64() - started, event.dwThreadId,
                           (LONG)context.Eax, row, owner, (LONG)stack30,
                           (LONG)stack34);
                    fflush(stdout);
                    events++;
                    if (events == 1 && pause_ms) {
                        fprintf(stderr, "captura congelada: pid=%lu, fila=%lu, pausa=%lu ms\n",
                                pid, row, pause_ms);
                        fflush(stderr);
                        Sleep(pause_ms);
                    }
                    if (!write_byte(process, original)) {
                        fprintf(stderr, "no se pudo restaurar breakpoint\n");
                        armed = 0;
                    }
                    context.Eip = (DWORD)(UINT_PTR)RETURN_ADDRESS;
                    if (events >= max_events) {
                        context.EFlags &= ~0x100;
                        done = 1;
                    } else {
                        context.EFlags |= 0x100;
                        stepping_thread = event.dwThreadId;
                    }
                    if (!SetThreadContext(thread, &context)) {
                        fprintf(stderr, "SetThreadContext: %lu\n", GetLastError());
                        armed = 0;
                    }
                } else {
                    context.EFlags &= ~0x100;
                    if (!write_byte(process, 0xCC) || !SetThreadContext(thread, &context)) {
                        fprintf(stderr, "no se pudo rearmar breakpoint\n");
                        armed = 0;
                    }
                    stepping_thread = 0;
                }
                if (thread) CloseHandle(thread);
            } else if (code != EXCEPTION_BREAKPOINT) {
                disposition = DBG_EXCEPTION_NOT_HANDLED;
            }
        } else if (event.dwDebugEventCode == CREATE_PROCESS_DEBUG_EVENT) {
            if (event.u.CreateProcessInfo.hFile) CloseHandle(event.u.CreateProcessInfo.hFile);
            if (event.u.CreateProcessInfo.hThread) CloseHandle(event.u.CreateProcessInfo.hThread);
            if (event.u.CreateProcessInfo.hProcess) CloseHandle(event.u.CreateProcessInfo.hProcess);
        } else if (event.dwDebugEventCode == LOAD_DLL_DEBUG_EVENT) {
            if (event.u.LoadDll.hFile) CloseHandle(event.u.LoadDll.hFile);
        }
        ContinueDebugEvent(event.dwProcessId, event.dwThreadId, disposition);
        if (process_exited) break;
    }
    write_byte(process, original);
    DebugActiveProcessStop(pid);
    CloseHandle(process);
    fprintf(stderr, "eventos=%u\n", events);
    return armed ? 0 : 1;
}
