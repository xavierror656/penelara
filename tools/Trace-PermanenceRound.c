/* Probe the context+8 rounding branch in an isolated 32-bit MotHor process. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const DWORD first = 0x00458DF0;
static const DWORD second = 0x00458E09;

static int read_mem(HANDLE process, DWORD address, void *data, SIZE_T size) {
    SIZE_T done = 0;
    return ReadProcessMemory(process, (LPCVOID)(UINT_PTR)address, data, size,
                             &done) && done == size;
}

static int write_mem(HANDLE process, DWORD address, const void *data, SIZE_T size) {
    SIZE_T done = 0;
    DWORD old = 0, unused = 0;
    LPVOID target = (LPVOID)(UINT_PTR)address;
    if (!VirtualProtectEx(process, target, size, PAGE_EXECUTE_READWRITE, &old))
        return 0;
    int ok = WriteProcessMemory(process, target, data, size, &done) && done == size;
    FlushInstructionCache(process, target, size);
    VirtualProtectEx(process, target, size, old, &unused);
    return ok;
}

int main(int argc, char **argv) {
    if ((argc != 3 && argc != 4 && argc != 5) || sizeof(void *) != 4) {
        fprintf(stderr, "uso: Trace-PermanenceRound.exe <MotHor PID> <segundos> [fila [subtotal]|natural [fila]] (x86)\n");
        return 2;
    }
    DWORD pid = (DWORD)strtoul(argv[1], NULL, 10);
    DWORD duration = (DWORD)strtoul(argv[2], NULL, 10) * 1000;
    int natural = argc >= 4 && strcmp(argv[3], "natural") == 0;
    int selected_row = -1;
    if (argc == 4 && !natural) selected_row = atoi(argv[3]);
    if (argc == 5) selected_row = atoi(natural ? argv[4] : argv[3]);
    LONG forced_subtotal = argc == 5 && !natural ? strtol(argv[4], NULL, 10) : -1;
    if (argc == 5 && !natural && forced_subtotal < 0) {
        fprintf(stderr, "subtotal must be nonnegative\n");
        return 2;
    }
    HANDLE process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ |
        PROCESS_VM_WRITE | PROCESS_VM_OPERATION, FALSE, pid);
    if (!process) {
        fprintf(stderr, "OpenProcess: %lu\n", GetLastError());
        return 1;
    }
    BYTE original[2];
    if (!read_mem(process, first, &original[0], 1) ||
        !read_mem(process, second, &original[1], 1) ||
        original[0] != 0x83 || original[1] != 0x03) {
        fprintf(stderr, "unexpected instruction bytes: %02X %02X\n",
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
    int phase = 0, failed = 0, armed = write_mem(process, first, "\xCC", 1);
    DWORD owner = 0, original_mode = 0, target_thread = 0, target_row = 0;
    DWORD stepping_thread = 0;
    if (!armed) failed = 1;
    puts("phase,thread,row,context,mode,ecx,original_ecx");
    fflush(stdout);
    ULONGLONG started = GetTickCount64();
    while (!failed && phase < 2 &&
           (stepping_thread || GetTickCount64() - started < duration)) {
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
            DWORD point = phase == 0 ? first : second;
            if ((code == EXCEPTION_BREAKPOINT && address == point) ||
                (code == EXCEPTION_SINGLE_STEP && event.dwThreadId == stepping_thread)) {
                HANDLE thread = OpenThread(THREAD_GET_CONTEXT | THREAD_SET_CONTEXT,
                                           FALSE, event.dwThreadId);
                CONTEXT context;
                ZeroMemory(&context, sizeof(context));
                context.ContextFlags = CONTEXT_CONTROL | CONTEXT_INTEGER;
                DWORD current_owner = 0, row = 0, mode = 0;
                if (!thread || !GetThreadContext(thread, &context)) {
                    fprintf(stderr, "failed to read thread context: %lu\n", GetLastError());
                    failed = 1;
                } else if (code == EXCEPTION_SINGLE_STEP) {
                    context.EFlags &= ~0x100;
                    stepping_thread = 0;
                    if (!write_mem(process, first, "\xCC", 1) ||
                        !SetThreadContext(thread, &context)) failed = 1;
                    else armed = 1;
                } else if (
                    !read_mem(process, context.Esp + 0x14, &current_owner, 4) ||
                    !read_mem(process, context.Esp + 0x6C, &row, 4) ||
                    !read_mem(process, current_owner + 8, &mode, 4)) {
                    fprintf(stderr, "failed to read breakpoint state: %lu\n", GetLastError());
                    failed = 1;
                } else if (phase == 0 && selected_row >= 0 && row != (DWORD)selected_row) {
                    if (!write_mem(process, first, &original[0], 1)) failed = 1;
                    armed = 0;
                    context.Eip = first;
                    context.EFlags |= 0x100;
                    stepping_thread = event.dwThreadId;
                    if (!failed && !SetThreadContext(thread, &context)) failed = 1;
                } else if (phase == 0 && mode != (DWORD)(natural ? 1 : 0)) {
                    fprintf(stderr, "unexpected initial context+8=%lu\n", mode);
                    failed = 1;
                } else if (phase == 1 && (event.dwThreadId != target_thread ||
                                           current_owner != owner || row != target_row || mode != 1)) {
                    fprintf(stderr, "second breakpoint belongs to a different calculation\n");
                    failed = 1;
                } else {
                    LONG original_ecx = (LONG)context.Ecx;
                    if (phase == 0 && forced_subtotal >= 0) context.Ecx = (DWORD)forced_subtotal;
                    printf("%d,%lu,%lu,%08lX,%lu,%ld,%ld\n", phase + 1,
                           event.dwThreadId, row, current_owner, mode,
                           (LONG)context.Ecx, original_ecx);
                    fflush(stdout);
                    if (phase == 0) {
                        owner = current_owner;
                        target_thread = event.dwThreadId;
                        target_row = row;
                        original_mode = mode;
                    }
                    if (!write_mem(process, point, &original[phase], 1)) failed = 1;
                    armed = 0;
                    if (!failed && phase == 0) {
                        DWORD forced = 1;
                        if ((!natural && !write_mem(process, owner + 8, &forced, 4)) ||
                            !write_mem(process, second, "\xCC", 1)) failed = 1;
                        else armed = 1;
                    }
                    if (!natural && !failed && phase == 1 &&
                        !write_mem(process, owner + 8, &original_mode, 4)) failed = 1;
                    context.Eip = point;
                    if (!failed && !SetThreadContext(thread, &context)) failed = 1;
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
    if (armed) {
        DWORD point = phase == 0 ? first : second;
        write_mem(process, point, &original[phase], 1);
    }
    if (owner && !natural) write_mem(process, owner + 8, &original_mode, 4);
    DebugActiveProcessStop(pid);
    CloseHandle(process);
    printf("events=%d failed=%d\n", phase, failed);
    return !failed && phase == 2 ? 0 : 1;
}
