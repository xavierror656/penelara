/* Observe both 0x459a50 returns in MotHor's split-day path. Build as x86. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>

static const DWORD points[2] = {0x00458D39, 0x00458D62};

static int read_bytes(HANDLE process, DWORD address, void *data, SIZE_T length) {
    SIZE_T count = 0;
    return ReadProcessMemory(process, (LPCVOID)(UINT_PTR)address, data, length,
                             &count) && count == length;
}

static int write_byte(HANDLE process, DWORD address, BYTE value) {
    DWORD old_protect = 0, unused = 0;
    SIZE_T count = 0;
    LPVOID target = (LPVOID)(UINT_PTR)address;
    if (!VirtualProtectEx(process, target, 1, PAGE_EXECUTE_READWRITE,
                          &old_protect)) return 0;
    int ok = WriteProcessMemory(process, target, &value, 1, &count) && count == 1;
    FlushInstructionCache(process, target, 1);
    VirtualProtectEx(process, target, 1, old_protect, &unused);
    return ok;
}

int main(int argc, char **argv) {
    if ((argc != 4 && argc != 5) || sizeof(void *) != 4) {
        fprintf(stderr, "uso: Trace-GapSplit.exe <MotHor PID> <segundos> <pausa-ms> [fila] (x86)\n");
        return 2;
    }
    DWORD pid = (DWORD)strtoul(argv[1], NULL, 10);
    DWORD duration_ms = (DWORD)strtoul(argv[2], NULL, 10) * 1000;
    DWORD pause_ms = (DWORD)strtoul(argv[3], NULL, 10);
    int target_row = argc == 5 ? atoi(argv[4]) : -1;
    HANDLE process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ |
        PROCESS_VM_WRITE | PROCESS_VM_OPERATION, FALSE, pid);
    if (!process) {
        fprintf(stderr, "OpenProcess: %lu\n", GetLastError());
        return 1;
    }
    BYTE original[2] = {0, 0};
    if (!read_bytes(process, points[0], &original[0], 1) ||
        !read_bytes(process, points[1], &original[1], 1) ||
        original[0] != 0x8B || original[1] != 0x8B) {
        fprintf(stderr, "bytes de retorno inesperados: %02X %02X\n",
                original[0], original[1]);
        CloseHandle(process);
        return 1;
    }
    if (!DebugActiveProcess(pid)) {
        fprintf(stderr, "DebugActiveProcess: %lu\n", GetLastError());
        CloseHandle(process);
        return 1;
    }
    DebugSetProcessKillOnExit(FALSE);
    int armed = write_byte(process, points[0], 0xCC);
    int phase = 0, failed = !armed;
    DWORD first_thread = 0, first_row = 0;
    DWORD stepping_thread = 0;
    ULONGLONG started = GetTickCount64();
    printf("phase,elapsed_ms,thread,eax,row,context,stack24,stack30,stack34,stack38,stack40\n");
    fflush(stdout);
    while (!failed && phase < 2 && (stepping_thread || GetTickCount64() - started < duration_ms)) {
        DEBUG_EVENT event;
        if (!WaitForDebugEvent(&event, 500)) {
            if (GetLastError() != ERROR_SEM_TIMEOUT) {
                fprintf(stderr, "WaitForDebugEvent: %lu\n", GetLastError());
                failed = 1;
            }
            continue;
        }
        DWORD disposition = DBG_CONTINUE;
        int exited = event.dwDebugEventCode == EXIT_PROCESS_DEBUG_EVENT;
        if (event.dwDebugEventCode == EXCEPTION_DEBUG_EVENT) {
            DWORD code = event.u.Exception.ExceptionRecord.ExceptionCode;
            DWORD address = (DWORD)(UINT_PTR)event.u.Exception.ExceptionRecord.ExceptionAddress;
            if ((code == EXCEPTION_BREAKPOINT && address == points[phase]) ||
                (code == EXCEPTION_SINGLE_STEP && event.dwThreadId == stepping_thread)) {
                HANDLE thread = OpenThread(THREAD_GET_CONTEXT | THREAD_SET_CONTEXT,
                                           FALSE, event.dwThreadId);
                CONTEXT context;
                ZeroMemory(&context, sizeof(context));
                context.ContextFlags = CONTEXT_CONTROL | CONTEXT_INTEGER;
                DWORD row = 0, owner = 0, locals[5] = {0};
                if (!thread || !GetThreadContext(thread, &context)) {
                    fprintf(stderr, "lectura de contexto: %lu\n", GetLastError());
                    failed = 1;
                    disposition = DBG_EXCEPTION_NOT_HANDLED;
                } else if (code == EXCEPTION_SINGLE_STEP) {
                    context.EFlags &= ~0x100;
                    armed = write_byte(process, points[0], 0xCC);
                    stepping_thread = 0;
                    if (!armed || !SetThreadContext(thread, &context)) {
                        fprintf(stderr, "no se pudo rearmar breakpoint\n");
                        failed = 1;
                    }
                } else if (
                    !read_bytes(process, context.Esp + 0x14, &owner, 4) ||
                    !read_bytes(process, context.Esp + 0x6C, &row, 4) ||
                    !read_bytes(process, context.Esp + 0x24, &locals[0], 4) ||
                    !read_bytes(process, context.Esp + 0x30, &locals[1], 4) ||
                    !read_bytes(process, context.Esp + 0x34, &locals[2], 4) ||
                    !read_bytes(process, context.Esp + 0x38, &locals[3], 4) ||
                    !read_bytes(process, context.Esp + 0x40, &locals[4], 4)) {
                    fprintf(stderr, "lectura de contexto/pila: %lu\n", GetLastError());
                    failed = 1;
                    disposition = DBG_EXCEPTION_NOT_HANDLED;
                } else {
                    if (phase == 0 && target_row >= 0 && row != (DWORD)target_row) {
                        if (!write_byte(process, points[0], original[0])) {
                            fprintf(stderr, "no se pudo restaurar breakpoint filtrado\n");
                            failed = 1;
                        }
                        armed = 0;
                        context.Eip = points[0];
                        context.EFlags |= 0x100;
                        stepping_thread = event.dwThreadId;
                        if (!SetThreadContext(thread, &context)) {
                            fprintf(stderr, "SetThreadContext filtrado: %lu\n", GetLastError());
                            failed = 1;
                        }
                        if (thread) CloseHandle(thread);
                        ContinueDebugEvent(event.dwProcessId, event.dwThreadId, disposition);
                        continue;
                    }
                    printf("%d,%llu,%lu,%ld,%lu,%08lX,%ld,%ld,%ld,%ld,%ld\n",
                           phase + 1, GetTickCount64() - started, event.dwThreadId,
                           (LONG)context.Eax, row, owner, (LONG)locals[0],
                           (LONG)locals[1], (LONG)locals[2], (LONG)locals[3],
                           (LONG)locals[4]);
                    fflush(stdout);
                    if (phase == 0) {
                        first_thread = event.dwThreadId;
                        first_row = row;
                        if (pause_ms) {
                            fprintf(stderr, "captura congelada: pid=%lu, fila=%lu, pausa=%lu ms\n",
                                    pid, row, pause_ms);
                            fflush(stderr);
                            Sleep(pause_ms);
                        }
                    } else if (event.dwThreadId != first_thread || row != first_row) {
                        fprintf(stderr, "segunda llamada de otra fila o hilo\n");
                        failed = 1;
                    }
                    if (!write_byte(process, points[phase], original[phase])) {
                        fprintf(stderr, "no se pudo restaurar breakpoint %d\n", phase + 1);
                        failed = 1;
                    }
                    armed = 0;
                    context.Eip = points[phase];
                    context.EFlags &= ~0x100;
                    if (!failed && phase == 0) {
                        armed = write_byte(process, points[1], 0xCC);
                        if (!armed) failed = 1;
                    }
                    if (!SetThreadContext(thread, &context)) {
                        fprintf(stderr, "SetThreadContext: %lu\n", GetLastError());
                        failed = 1;
                    }
                    phase++;
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
        if (exited) break;
    }
    if (armed) write_byte(process, points[phase], original[phase]);
    DebugActiveProcessStop(pid);
    CloseHandle(process);
    fprintf(stderr, "eventos=%d\n", phase);
    return !failed && phase == 2 ? 0 : 1;
}
