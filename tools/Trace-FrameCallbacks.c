/* Trace XML frame callbacks and mode initialization in isolated MotHor. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <tlhelp32.h>
#include <stdio.h>
#include <stdlib.h>
#include <wchar.h>

static const DWORD points[5] = {0x00419E25, 0x00419F30, 0x004261AA,
                                0x00423BA4, 0x004206F0};
static const char *names[5] = {"tramo", "marcos", "mode", "tm_parse", "text"};

static int read_mem(HANDLE process, DWORD address, void *data, SIZE_T size) {
    SIZE_T done = 0;
    return ReadProcessMemory(process, (LPCVOID)(UINT_PTR)address, data, size,
                             &done) && done == size;
}

static DWORD read_u32(HANDLE process, DWORD address) {
    DWORD value = 0;
    read_mem(process, address, &value, 4);
    return value;
}

static int write_byte(HANDLE process, DWORD address, BYTE value) {
    DWORD old = 0, unused = 0;
    SIZE_T done = 0;
    LPVOID target = (LPVOID)(UINT_PTR)address;
    if (!VirtualProtectEx(process, target, 1, PAGE_EXECUTE_READWRITE, &old))
        return 0;
    int ok = WriteProcessMemory(process, target, &value, 1, &done) && done == 1;
    FlushInstructionCache(process, target, 1);
    VirtualProtectEx(process, target, 1, old, &unused);
    return ok;
}

static DWORD find_motor(const wchar_t *marker) {
    HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (snapshot == INVALID_HANDLE_VALUE) return 0;
    PROCESSENTRY32W item;
    ZeroMemory(&item, sizeof(item));
    item.dwSize = sizeof(item);
    DWORD result = 0;
    if (Process32FirstW(snapshot, &item)) do {
        if (_wcsicmp(item.szExeFile, L"MotHor.exe") != 0) continue;
        HANDLE process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE,
                                     item.th32ProcessID);
        if (!process) continue;
        wchar_t path[MAX_PATH * 2];
        DWORD length = sizeof(path) / sizeof(path[0]);
        if (QueryFullProcessImageNameW(process, 0, path, &length) &&
            wcsstr(path, marker)) result = item.th32ProcessID;
        CloseHandle(process);
        if (result) break;
    } while (Process32NextW(snapshot, &item));
    CloseHandle(snapshot);
    return result;
}

int wmain(int argc, wchar_t **argv) {
    if (argc != 3 || sizeof(void *) != 4) {
        fwprintf(stderr, L"usage: Trace-FrameCallbacks.exe <run-marker> <seconds> (x86)\n");
        return 2;
    }
    DWORD duration = (DWORD)wcstoul(argv[2], NULL, 10) * 1000;
    ULONGLONG started = GetTickCount64();
    DWORD pid = 0;
    while (!pid && GetTickCount64() - started < duration) {
        pid = find_motor(argv[1]);
        if (!pid) Sleep(1);
    }
    if (!pid) {
        fprintf(stderr, "matching MotHor did not start\n");
        return 1;
    }
    HANDLE process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ |
        PROCESS_VM_WRITE | PROCESS_VM_OPERATION, FALSE, pid);
    if (!process) {
        fprintf(stderr, "OpenProcess: %lu\n", GetLastError());
        return 1;
    }
    if (!DebugActiveProcess(pid)) {
        fprintf(stderr, "DebugActiveProcess: %lu\n", GetLastError());
        CloseHandle(process);
        return 1;
    }
    DebugSetProcessKillOnExit(FALSE);
    BYTE original[5] = {0};
    int armed[5] = {0}, failed = 0, events = 0;
    for (int i = 0; i < 5; ++i) {
        if (!read_mem(process, points[i], &original[i], 1) ||
            !write_byte(process, points[i], 0xCC)) failed = 1;
        else armed[i] = 1;
    }
    printf("event,ms,pid,thread,point,elementId,loader,object,listHead,listCount,field1c470,buffer0,flag65ba4,flag65ba8,modelMode,modeToWrite,nonzeroDc,path\n");
    fflush(stdout);
    DWORD stepping_thread = 0;
    int stepping_point = -1;
    while (!failed && GetTickCount64() - started < duration) {
        DEBUG_EVENT event;
        if (!WaitForDebugEvent(&event, 200)) {
            if (GetLastError() != ERROR_SEM_TIMEOUT) failed = 1;
            continue;
        }
        DWORD disposition = DBG_CONTINUE;
        int exited = event.dwDebugEventCode == EXIT_PROCESS_DEBUG_EVENT;
        if (event.dwDebugEventCode == EXCEPTION_DEBUG_EVENT) {
            DWORD code = event.u.Exception.ExceptionRecord.ExceptionCode;
            DWORD address = (DWORD)(UINT_PTR)event.u.Exception.ExceptionRecord.ExceptionAddress;
            int hit = -1;
            for (int i = 0; i < 5; ++i)
                if (code == EXCEPTION_BREAKPOINT && address == points[i]) hit = i;
            if (hit >= 0 || (code == EXCEPTION_SINGLE_STEP &&
                             event.dwThreadId == stepping_thread)) {
                HANDLE thread = OpenThread(THREAD_GET_CONTEXT | THREAD_SET_CONTEXT,
                                           FALSE, event.dwThreadId);
                CONTEXT context;
                ZeroMemory(&context, sizeof(context));
                context.ContextFlags = CONTEXT_CONTROL | CONTEXT_INTEGER;
                if (!thread || !GetThreadContext(thread, &context)) {
                    failed = 1;
                } else if (hit >= 0) {
                    DWORD loader = hit == 2 ? 0 : hit == 4 ? context.Ecx : context.Edi;
                    DWORD element_id = loader ? read_u32(process, loader + 0x28) : 0;
                    DWORD object = hit == 2 ? context.Edi :
                        hit == 1 ? context.Esi : read_u32(process, loader + 0x64EE4);
                    DWORD list_head = object ? read_u32(process, object + 0x135BD4) : 0;
                    DWORD list_count = object ? read_u32(process, object + 0x135BDC) : 0;
                    DWORD field = loader ? read_u32(process, loader + 0x1C470) : 0;
                    BYTE buffer = 0;
                    if (loader) read_mem(process, loader + 0x1BFDE, &buffer, 1);
                    DWORD flag_a = loader ? read_u32(process, loader + 0x65BA4) : 0;
                    DWORD flag_b = loader ? read_u32(process, loader + 0x65BA8) : 0;
                    DWORD model_mode = hit == 2 ?
                        read_u32(process, context.Eax + 0xAFC93) & 0xFF : 0;
                    DWORD nonzero_dc = 0;
                    if (hit == 2) {
                        DWORD node = list_head;
                        for (DWORD index = 0; node && index < list_count && index < 10000;
                             ++index) {
                            if (read_u32(process, node + 0xDC)) ++nonzero_dc;
                            node = read_u32(process, node);
                        }
                    }
                    if (hit != 4 || (element_id >= 0x60 && element_id <= 0x75)) {
                        char path[96] = {0};
                        DWORD path_ptr = loader ? read_u32(process, loader + 0x24) : 0;
                        if (path_ptr) read_mem(process, path_ptr, path, sizeof(path) - 1);
                        for (int k = 0; k < (int)sizeof(path); ++k)
                            if (path[k] == ',' || path[k] == '\r' || path[k] == '\n')
                                path[k] = ';';
                        printf("%d,%llu,%lu,%lu,%s,%lu,%08lX,%08lX,%08lX,%lu,%lu,%u,%lu,%lu,%lu,%lu,%lu,%s\n",
                               ++events, GetTickCount64() - started, pid,
                               event.dwThreadId, names[hit], element_id, loader, object,
                               list_head, list_count, field, buffer, flag_a,
                               flag_b, model_mode,
                               hit == 2 ? context.Ecx & 0xFF : 0, nonzero_dc, path);
                        fflush(stdout);
                    }
                    if (!write_byte(process, points[hit], original[hit])) failed = 1;
                    armed[hit] = 0;
                    context.Eip = points[hit];
                    context.EFlags |= 0x100;
                    stepping_thread = event.dwThreadId;
                    stepping_point = hit;
                    if (!failed && !SetThreadContext(thread, &context)) failed = 1;
                } else {
                    context.EFlags &= ~0x100;
                    if (!write_byte(process, points[stepping_point], 0xCC) ||
                        !SetThreadContext(thread, &context)) failed = 1;
                    else armed[stepping_point] = 1;
                    stepping_thread = 0;
                    stepping_point = -1;
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
    for (int i = 0; i < 5; ++i)
        if (armed[i]) write_byte(process, points[i], original[i]);
    DebugActiveProcessStop(pid);
    CloseHandle(process);
    printf("events=%d failed=%d\n", events, failed);
    return failed ? 1 : 0;
}
