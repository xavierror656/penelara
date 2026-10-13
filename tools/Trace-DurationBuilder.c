/* Record the source count passed into MotHor's duration-aware builder. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>

static const DWORD point = 0x0043A2D4;

static int read_bytes(HANDLE process, DWORD address, void *buffer, SIZE_T size) {
    SIZE_T done = 0;
    return ReadProcessMemory(process, (LPCVOID)(UINT_PTR)address, buffer,
                             size, &done) && done == size;
}

static int write_byte(HANDLE process, DWORD address, BYTE value) {
    DWORD protection = 0, ignored = 0;
    SIZE_T done = 0;
    LPVOID target = (LPVOID)(UINT_PTR)address;
    if (!VirtualProtectEx(process, target, 1, PAGE_EXECUTE_READWRITE,
                          &protection)) return 0;
    int ok = WriteProcessMemory(process, target, &value, 1, &done) && done == 1;
    FlushInstructionCache(process, target, 1);
    VirtualProtectEx(process, target, 1, protection, &ignored);
    return ok;
}

int main(int argc, char **argv) {
    if (argc != 3 || sizeof(void *) != 4) {
        fprintf(stderr, "uso: Trace-DurationBuilder.exe <MotHor PID> <segundos> (x86)\n");
        return 2;
    }
    DWORD pid = (DWORD)strtoul(argv[1], NULL, 10);
    DWORD duration = (DWORD)strtoul(argv[2], NULL, 10) * 1000;
    HANDLE process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ |
        PROCESS_VM_WRITE | PROCESS_VM_OPERATION, FALSE, pid);
    if (!process) {
        fprintf(stderr, "OpenProcess: %lu\n", GetLastError());
        return 1;
    }
    BYTE original = 0;
    if (!read_bytes(process, point, &original, 1) || original != 0xE8) {
        fprintf(stderr, "unexpected call byte %02X\n", original);
        CloseHandle(process);
        return 1;
    }
    if (!DebugActiveProcess(pid)) {
        fprintf(stderr, "DebugActiveProcess: %lu\n", GetLastError());
        CloseHandle(process);
        return 1;
    }
    DebugSetProcessKillOnExit(FALSE);
    int failed = !write_byte(process, point, 0xCC);
    int armed = !failed, events = 0;
    DWORD stepping = 0;
    puts("event,thread,source,base_count,override_factor,builder_count");
    fflush(stdout);
    ULONGLONG started = GetTickCount64();
    while (!failed && (stepping || GetTickCount64() - started < duration)) {
        DEBUG_EVENT event;
        if (!WaitForDebugEvent(&event, 500)) {
            if (GetLastError() != ERROR_SEM_TIMEOUT) failed = 1;
            continue;
        }
        DWORD disposition = DBG_CONTINUE;
        int exited = event.dwDebugEventCode == EXIT_PROCESS_DEBUG_EVENT;
        if (event.dwDebugEventCode == EXCEPTION_DEBUG_EVENT) {
            DWORD code = event.u.Exception.ExceptionRecord.ExceptionCode;
            DWORD address = (DWORD)(UINT_PTR)event.u.Exception.ExceptionRecord.ExceptionAddress;
            if ((code == EXCEPTION_BREAKPOINT && address == point) ||
                (code == EXCEPTION_SINGLE_STEP && event.dwThreadId == stepping)) {
                HANDLE thread = OpenThread(THREAD_GET_CONTEXT | THREAD_SET_CONTEXT,
                                           FALSE, event.dwThreadId);
                CONTEXT state;
                ZeroMemory(&state, sizeof(state));
                state.ContextFlags = CONTEXT_CONTROL | CONTEXT_INTEGER;
                if (!thread || !GetThreadContext(thread, &state)) {
                    fprintf(stderr, "GetThreadContext: %lu\n", GetLastError());
                    failed = 1;
                } else if (code == EXCEPTION_SINGLE_STEP) {
                    state.EFlags &= ~0x100;
                    stepping = 0;
                    if (!write_byte(process, point, 0xCC) ||
                        !SetThreadContext(thread, &state)) failed = 1;
                    else armed = 1;
                } else {
                    BYTE base_count = 0, override_factor = 0;
                    if (!read_bytes(process, state.Esi + 0x310, &base_count, 1) ||
                        !read_bytes(process, state.Esi + 0x312, &override_factor, 1)) {
                        fprintf(stderr, "ReadProcessMemory source: %lu\n", GetLastError());
                        failed = 1;
                    } else {
                        ++events;
                        printf("%d,%lu,%08lX,%u,%u,%lu\n", events,
                               event.dwThreadId, state.Esi, base_count,
                               override_factor, state.Ebp);
                        fflush(stdout);
                    }
                    if (!write_byte(process, point, original)) failed = 1;
                    armed = 0;
                    state.Eip = point;
                    state.EFlags |= 0x100;
                    stepping = event.dwThreadId;
                    if (!failed && !SetThreadContext(thread, &state)) failed = 1;
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
    if (armed) write_byte(process, point, original);
    DebugActiveProcessStop(pid);
    CloseHandle(process);
    printf("events=%d failed=%d\n", events, failed);
    return failed ? 1 : 0;
}
