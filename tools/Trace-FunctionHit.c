/* Count hits at an x86 MotHor instruction without altering registers. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int read_u32(HANDLE process, DWORD address, DWORD *value) {
    SIZE_T done = 0;
    return ReadProcessMemory(process, (LPCVOID)(UINT_PTR)address, value,
                             sizeof(*value), &done) && done == sizeof(*value);
}

static void dump_entries(HANDLE process, const char *path) {
    DWORD app = 0, bridge = 0, entries = 0, count = 0;
    if (!read_u32(process, 0x4932C0, &app) || !app ||
        !read_u32(process, app + 0xDC, &bridge) || !bridge ||
        !read_u32(process, bridge + 0x135BC4, &entries) || !entries ||
        !read_u32(process, bridge + 0x135BC8, &count) || count > 4096) {
        puts("dump_error=pointers");
        return;
    }
    SIZE_T size = (SIZE_T)count * 0x288, done = 0;
    BYTE *buffer = (BYTE *)malloc(size ? size : 1);
    if (!buffer || !ReadProcessMemory(process, (LPCVOID)(UINT_PTR)entries,
                                       buffer, size, &done) || done != size) {
        puts("dump_error=read");
        free(buffer);
        return;
    }
    FILE *file = fopen(path, "wb");
    if (!file || fwrite(buffer, 1, size, file) != size) {
        puts("dump_error=write");
    } else {
        printf("dump_count=%lu dump_pointer=%08lX dump_bytes=%llu\n",
               count, entries, (unsigned long long)size);
    }
    if (file) fclose(file);
    free(buffer);
    char source_path[1024];
    if (snprintf(source_path, sizeof(source_path), "%s.sources.bin", path) >=
        (int)sizeof(source_path)) return;
    DWORD node = 0;
    if (!read_u32(process, bridge + 0xD047C, &node)) return;
    FILE *sources = fopen(source_path, "wb");
    if (!sources) return;
    int source_count = 0;
    while (node && source_count < 256) {
        BYTE record[0x334];
        if (!ReadProcessMemory(process, (LPCVOID)(UINT_PTR)node, record,
                               sizeof(record), &done) || done != sizeof(record) ||
            fwrite(record, 1, sizeof(record), sources) != sizeof(record)) break;
        ++source_count;
        node = *(DWORD *)record;
    }
    fclose(sources);
    printf("dump_source_count=%d dump_source_terminated=%d\n",
           source_count, node == 0);
}

static int byte_at(HANDLE process, DWORD address, BYTE *value) {
    SIZE_T done = 0;
    return ReadProcessMemory(process, (LPCVOID)(UINT_PTR)address, value, 1,
                             &done) && done == 1;
}

static int put_byte(HANDLE process, DWORD address, BYTE value) {
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

static int release_entry(HANDLE process) {
    const DWORD address = 0x4722AA;
    const BYTE expected[5] = {0xEB, 0xFE, 0x06, 0x00, 0x00};
    const BYTE original[2] = {0xE8, 0x45};
    BYTE actual[5] = {0};
    SIZE_T done = 0;
    DWORD protection = 0, ignored = 0;
    if (!ReadProcessMemory(process, (LPCVOID)(UINT_PTR)address, actual,
                           sizeof(actual), &done) || done != sizeof(actual) ||
        memcmp(actual, expected, sizeof(actual)) != 0) return 0;
    if (!VirtualProtectEx(process, (LPVOID)(UINT_PTR)address, sizeof(original),
                          PAGE_EXECUTE_READWRITE, &protection)) return 0;
    int ok = WriteProcessMemory(process, (LPVOID)(UINT_PTR)address, original,
                                sizeof(original), &done) && done == sizeof(original);
    FlushInstructionCache(process, (LPCVOID)(UINT_PTR)address, sizeof(original));
    VirtualProtectEx(process, (LPVOID)(UINT_PTR)address, sizeof(original),
                     protection, &ignored);
    return ok;
}

int main(int argc, char **argv) {
    if (argc < 4 || argc > 6 || sizeof(void *) != 4) {
        fprintf(stderr, "uso: Trace-FunctionHit.exe <PID> <VA> <segundos> [dump] [--release-entry] (x86)\n");
        return 2;
    }
    const char *dump_path = NULL;
    int release = 0;
    for (int index = 4; index < argc; ++index) {
        if (strcmp(argv[index], "--release-entry") == 0) release = 1;
        else if (!dump_path) dump_path = argv[index];
        else {
            fprintf(stderr, "too many dump paths\n");
            return 2;
        }
    }
    DWORD pid = (DWORD)strtoul(argv[1], NULL, 10);
    DWORD point = (DWORD)strtoul(argv[2], NULL, 0);
    DWORD duration = (DWORD)strtoul(argv[3], NULL, 10) * 1000;
    HANDLE process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ |
        PROCESS_VM_WRITE | PROCESS_VM_OPERATION, FALSE, pid);
    if (!process) {
        fprintf(stderr, "OpenProcess: %lu\n", GetLastError());
        return 1;
    }
    BYTE original = 0;
    if (!byte_at(process, point, &original)) {
        fprintf(stderr, "ReadProcessMemory: %lu\n", GetLastError());
        CloseHandle(process);
        return 1;
    }
    if (!DebugActiveProcess(pid)) {
        fprintf(stderr, "DebugActiveProcess: %lu\n", GetLastError());
        CloseHandle(process);
        return 1;
    }
    DebugSetProcessKillOnExit(FALSE);
    int failed = !put_byte(process, point, 0xCC), armed = !failed, hits = 0;
    if (!failed && release) {
        failed = !release_entry(process);
        printf("release_entry=%d\n", !failed);
    }
    unsigned long eax_small[17] = {0}, eax_other = 0;
    unsigned long type3_checks = 0, type3_rejected = 0;
    unsigned long target_over = 0, target_one_short = 0;
    unsigned long target_two_short = 0, target_exact = 0, target_other = 0;
    unsigned long type3_unplaced = 0;
    unsigned long type3_gate = 0, type3_gate_skipped = 0;
    unsigned long type3_increment = 0, type3_increment_small[17] = {0};
    unsigned long serialized_complementarias = 0;
    unsigned long completion_quota_type3 = 0, completion_quota_met = 0;
    unsigned long duration_t_matrix_writes = 0, duration_m_matrix_writes = 0;
    unsigned long duration_matrix_reads = 0, duration_matrix_three = 0;
    unsigned long duration_matrix_two = 0, duration_matrix_two_type[5] = {0};
    unsigned long duration_matrix_three_type[5] = {0};
    DWORD stepping = 0;
    printf("point=0x%08lX original=0x%02X\n", point, original);
    puts("hit,thread,return,row,eax,ecx,edx,ebp,esi,edi");
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
                    failed = 1;
                } else if (code == EXCEPTION_SINGLE_STEP) {
                    state.EFlags &= ~0x100;
                    stepping = 0;
                    if (!put_byte(process, point, 0xCC) ||
                        !SetThreadContext(thread, &state)) failed = 1;
                    else armed = 1;
                } else {
                    ++hits;
                    if (hits == 1 && dump_path) dump_entries(process, dump_path);
                    if (state.Eax <= 16) ++eax_small[state.Eax];
                    else ++eax_other;
                    if (point == 0x4492A0 || point == 0x44939E) {
                        BYTE entry_type = 0;
                        if (byte_at(process, state.Ebx + 0x271, &entry_type) &&
                            entry_type == 3) {
                            ++type3_checks;
                            LONG total = (LONG)(point == 0x44939E ? state.Edx : state.Ecx);
                            LONG limit = (LONG)(point == 0x44939E ? state.Ecx : state.Eax);
                            if (total > limit) ++type3_rejected;
                            if (type3_checks <= 20)
                                printf("type3_check=%lu entry=%08lX daily_plus_candidate=%ld limit=%ld rejected=%d\n",
                                       type3_checks, state.Ebx, total, limit, total > limit);
                        }
                    }
                    if (point == 0x4454BA) {
                        BYTE entry_type = 0, occupied = 0;
                        if (byte_at(process, state.Ebp + 0x271, &entry_type) &&
                            entry_type == 3) {
                            ++type3_checks;
                            if (byte_at(process, state.Eax + state.Ebx, &occupied) && occupied)
                                ++type3_rejected;
                            if (type3_checks <= 20)
                                printf("type3_occupied=%lu entry=%08lX slot=%ld occupied=%u\n",
                                       type3_checks, state.Ebp, (LONG)state.Ebx, occupied);
                        }
                    }
                    if (point == 0x4454EA || point == 0x44550A ||
                        point == 0x445568 || point == 0x445584) {
                        BYTE entry_type = 0;
                        if (byte_at(process, state.Ebp + 0x271, &entry_type) &&
                            entry_type == 3) {
                            LONG result = (LONG)(point == 0x44550A ? state.Edx : state.Eax);
                            int rejected = point == 0x4454EA || point == 0x44550A
                                ? result == -1 : point == 0x445568 ? result == 0 : result != 1;
                            ++type3_checks;
                            if (rejected) ++type3_rejected;
                            if (type3_checks <= 20)
                                printf("type3_candidate=%lu point=%08lX entry=%08lX result=%ld rejected=%d\n",
                                       type3_checks, point, state.Ebp, result, rejected);
                        }
                    }
                    if (point == 0x448416 || point == 0x4484DA ||
                        point == 0x4484EF || point == 0x44850A) {
                        BYTE entry_type = 0;
                        if (byte_at(process, state.Esi + 0x271, &entry_type) &&
                            entry_type == 3) {
                            ++type3_checks;
                            if (!state.Eax) ++type3_rejected;
                            if (type3_checks <= 20) {
                                BYTE field26 = 0, field74 = 0, field268 = 0, field26a = 0;
                                DWORD field30 = 0, field78 = 0, field84 = 0;
                                DWORD field158 = 0, field22c = 0, field244 = 0;
                                BYTE field240 = 0;
                                byte_at(process, state.Esi + 0x26, &field26);
                                byte_at(process, state.Esi + 0x74, &field74);
                                read_u32(process, state.Esi + 0x30, &field30);
                                read_u32(process, state.Esi + 0x78, &field78);
                                read_u32(process, state.Esi + 0x84, &field84);
                                read_u32(process, state.Esi + 0x158, &field158);
                                read_u32(process, state.Esi + 0x22C, &field22c);
                                read_u32(process, state.Esi + 0x244, &field244);
                                byte_at(process, state.Esi + 0x240, &field240);
                                byte_at(process, state.Esi + 0x268, &field268);
                                byte_at(process, state.Esi + 0x26A, &field26a);
                                printf("type3_inner=%lu point=%08lX entry=%08lX result=%ld rejected=%d field26=%u field74=%u field30=%08lX field78=%08lX field84=%08lX field158=%08lX field22c=%08lX field240=%u field244=%08lX field268=%u field26a=%u\n",
                                       type3_checks, point, state.Esi,
                                       (LONG)state.Eax, !state.Eax, field26, field74,
                                       field30, field78, field84, field158, field22c,
                                       field240, field244, field268, field26a);
                            }
                        }
                    }
                    if (point == 0x448D89) {
                        BYTE entry_type = 0, mode = 0;
                        if (byte_at(process, state.Ebx + 0x271, &entry_type) &&
                            entry_type == 3) {
                            ++type3_checks;
                            if ((LONG)state.Edx > 1) ++type3_rejected;
                            byte_at(process, state.Ebx + 0x74, &mode);
                            if (type3_checks <= 20)
                                printf("type3_runs=%lu entry=%08lX field74=%u runs=%ld reject_here=%d\n",
                                       type3_checks, state.Ebx, mode,
                                       (LONG)state.Edx, (LONG)state.Edx > 1);
                        }
                    }
                    if (point == 0x44917B) {
                        BYTE entry_type = 0;
                        if (byte_at(process, state.Ebx + 0x271, &entry_type) &&
                            entry_type == 3) {
                            LONG proposed = (LONG)state.Ecx, target = (LONG)state.Edx;
                            ++type3_checks;
                            if (proposed > target) ++target_over;
                            else if (proposed + 1 == target) ++target_one_short;
                            else if (proposed + 2 == target) ++target_two_short;
                            else if (proposed == target) ++target_exact;
                            else ++target_other;
                            if (type3_checks <= 20)
                                printf("type3_target=%lu entry=%08lX proposed=%ld target=%ld remaining=%ld\n",
                                       type3_checks, state.Ebx, proposed, target,
                                       target - proposed);
                        }
                    }
                    if (point == 0x43A2D4) {
                        BYTE base_count = 0, override_factor = 0;
                        if (byte_at(process, state.Esi + 0x310, &base_count) &&
                            byte_at(process, state.Esi + 0x312, &override_factor))
                            printf("builder_hit=%d source=%08lX base_count=%u override_factor=%u builder_count=%lu\n",
                                   hits, state.Esi, base_count, override_factor,
                                   state.Ebp);
                    }
                    if (point == 0x4621B4) {
                        BYTE entry_type = 0, old_status = 0, adjusted_total = 0;
                        DWORD global_unplaced = 0;
                        if (byte_at(process, state.Esi + 0x271, &entry_type) &&
                            entry_type == 3) {
                            ++type3_unplaced;
                            byte_at(process, state.Esi + 0x26E, &old_status);
                            byte_at(process, state.Ecx, &adjusted_total);
                            read_u32(process, 0x493234, &global_unplaced);
                            if (type3_unplaced <= 20)
                                printf("type3_unplaced=%lu entry=%08lX old_status=%u adjusted_total=%u global_unplaced=%lu\n",
                                       type3_unplaced, state.Esi, old_status,
                                       adjusted_total, global_unplaced);
                        }
                    }
                    if (point == 0x4621A9) {
                        BYTE entry_type = 0, before = 0;
                        if (byte_at(process, state.Esi + 0x271, &entry_type) &&
                            entry_type == 3) {
                            ++type3_increment;
                            byte_at(process, state.Ecx, &before);
                            if (type3_increment <= 20)
                                printf("type3_adjustment=%lu entry=%08lX amount=%u before=%u after=%u\n",
                                       type3_increment, state.Esi, state.Eax & 0xFF,
                                       before, (before + (state.Eax & 0xFF)) & 0xFF);
                        }
                    }
                    if (point == 0x4402BF) {
                        BYTE entry_type = 0;
                        if (byte_at(process, state.Esi + 0x271, &entry_type) &&
                            entry_type == 3) {
                            ++type3_gate;
                            if ((LONG)state.Ecx >= (LONG)state.Eax)
                                ++type3_gate_skipped;
                            if (type3_gate <= 20) {
                                BYTE day = 0, primary = 0, adjustment = 0;
                                DWORD first = 0, second = 0;
                                int values_read = byte_at(process, state.Esi + 0x275, &day) &&
                                    read_u32(process, state.Edi + 0xCAA0 + day * 4, &first) &&
                                    read_u32(process, state.Edi + 0xCAB8 + day * 4, &second) &&
                                    byte_at(process, first + (WORD)state.Edx, &primary) &&
                                    byte_at(process, second + (WORD)state.Edx, &adjustment);
                                printf("type3_gate=%lu entry=%08lX sum=%ld target=%ld skip=%d primary=%u adjustment=%u values_read=%d\n",
                                       type3_gate, state.Esi, (LONG)state.Ecx,
                                       (LONG)state.Eax,
                                       (LONG)state.Ecx >= (LONG)state.Eax,
                                       primary, adjustment, values_read);
                            }
                        }
                    }
                    if (point == 0x44D5B9) {
                        BYTE entry_type = 0;
                        if (byte_at(process, state.Edi + 0x271, &entry_type) &&
                            entry_type == 3) {
                            unsigned amount = state.Edx & 0xFF;
                            ++type3_increment;
                            if (amount <= 16) ++type3_increment_small[amount];
                            if (type3_increment <= 20)
                                printf("type3_increment=%lu entry=%08lX amount=%u record=%ld\n",
                                       type3_increment, state.Edi, amount,
                                       (LONG)state.Ecx);
                        }
                    }
                    if (point == 0x415267) {
                        DWORD return_address = 0, second_arg = 0, third_arg = 0;
                        BYTE entry_type = 0;
                        WORD source_index = 0;
                        SIZE_T done = 0;
                        ++serialized_complementarias;
                        read_u32(process, state.Ebp + 4, &return_address);
                        read_u32(process, state.Ebp + 12, &second_arg);
                        read_u32(process, state.Ebp + 16, &third_arg);
                        byte_at(process, state.Ebx + 0x271, &entry_type);
                        ReadProcessMemory(process,
                                          (LPCVOID)(UINT_PTR)(state.Ebx + 0x264),
                                          &source_index, sizeof(source_index), &done);
                        if (serialized_complementarias <= 30)
                            printf("serialized_complementaria=%lu caller=%08lX entry=%08lX type=%u source=%u arg2=%lu arg3=%lu\n",
                                   serialized_complementarias, return_address,
                                   state.Ebx, entry_type, source_index,
                                   second_arg, third_arg);
                    }
                    if (point == 0x4620D5) {
                        BYTE entry_type = 0, source_index = 0, day = 0;
                        if (byte_at(process, state.Esi + 0x271, &entry_type) &&
                            entry_type == 3) {
                            ++completion_quota_type3;
                            if ((LONG)state.Eax >= (LONG)state.Ecx)
                                ++completion_quota_met;
                            byte_at(process, state.Esi + 0x264, &source_index);
                            byte_at(process, state.Esi + 0x275, &day);
                            if (completion_quota_type3 <= 30)
                                printf("completion_quota_type3=%lu entry=%08lX source=%u day=%u total=%ld target=%ld met=%d\n",
                                       completion_quota_type3, state.Esi,
                                       source_index, day, (LONG)state.Eax,
                                       (LONG)state.Ecx,
                                       (LONG)state.Eax >= (LONG)state.Ecx);
                        }
                    }
                    if (point == 0x4161F9) {
                        BYTE previous = 0;
                        DWORD target_address = state.Esi + state.Ecx + 4;
                        ++duration_t_matrix_writes;
                        byte_at(process, target_address, &previous);
                        if (duration_t_matrix_writes <= 30)
                            printf("duration_t_matrix_write=%lu address=%08lX matrix=%08lX index=%ld frame_offset=%ld day=%ld slot=%ld previous=%u\n",
                                   duration_t_matrix_writes, target_address,
                                   state.Esi + 4, (LONG)state.Ecx,
                                   (LONG)state.Edi, (LONG)state.Edx,
                                   (LONG)state.Eax, previous);
                    }
                    if (point == 0x4161EA) {
                        BYTE previous = 0;
                        DWORD target_address = state.Esi + state.Ecx + 4;
                        ++duration_m_matrix_writes;
                        byte_at(process, target_address, &previous);
                        if (duration_m_matrix_writes <= 30)
                            printf("duration_m_matrix_write=%lu address=%08lX matrix=%08lX index=%ld frame_offset=%ld day=%ld slot=%ld previous=%u\n",
                                   duration_m_matrix_writes, target_address,
                                   state.Esi + 4, (LONG)state.Ecx,
                                   (LONG)state.Edi, (LONG)state.Edx,
                                   (LONG)state.Eax, previous);
                    }
                    if (point == 0x44D357) {
                        BYTE entry_type = 0;
                        unsigned amount = state.Eax & 0xFF;
                        ++duration_matrix_reads;
                        if (amount == 2) {
                            ++duration_matrix_two;
                            byte_at(process, state.Edi + 0x271, &entry_type);
                            ++duration_matrix_two_type[entry_type <= 3 ? entry_type : 4];
                            if (duration_matrix_two <= 30)
                                printf("duration_matrix_two=%lu entry=%08lX type=%u frame=%ld day=%ld slot=%ld\n",
                                       duration_matrix_two, state.Edi,
                                       entry_type, (LONG)state.Ebp,
                                       (LONG)state.Ebx, (LONG)state.Edx);
                        }
                        if (amount == 3) {
                            ++duration_matrix_three;
                            byte_at(process, state.Edi + 0x271, &entry_type);
                            ++duration_matrix_three_type[entry_type <= 3 ? entry_type : 4];
                            if (duration_matrix_three <= 30)
                                printf("duration_matrix_three=%lu entry=%08lX type=%u frame=%ld day=%ld slot=%ld\n",
                                       duration_matrix_three, state.Edi,
                                       entry_type, (LONG)state.Ebp,
                                       (LONG)state.Ebx, (LONG)state.Edx);
                        }
                    }
                    if (hits <= 20) {
                        DWORD caller = 0, row = 0;
                        SIZE_T done = 0;
                        ReadProcessMemory(process, (LPCVOID)(UINT_PTR)state.Esp,
                                          &caller, sizeof(caller), &done);
                        if (point == 0x458DF0)
                            ReadProcessMemory(process, (LPCVOID)(UINT_PTR)(state.Esp + 0x6C),
                                              &row, sizeof(row), &done);
                        printf("%d,%lu,%08lX,%lu,%08lX,%08lX,%08lX,%08lX,%08lX,%08lX\n",
                               hits, event.dwThreadId, caller, row, state.Eax, state.Ecx,
                               state.Edx, state.Ebp, state.Esi, state.Edi);
                        fflush(stdout);
                    }
                    if (!put_byte(process, point, original)) failed = 1;
                    armed = 0;
                    state.Eip = point;
                    state.EFlags |= 0x100;
                    stepping = event.dwThreadId;
                    if (!failed && !SetThreadContext(thread, &state)) failed = 1;
                }
                if (thread) CloseHandle(thread);
            } else if (code != EXCEPTION_BREAKPOINT) {
                printf("exception code=%08lX address=%08lX first_chance=%lu\n",
                       code, address, event.u.Exception.dwFirstChance);
                disposition = DBG_EXCEPTION_NOT_HANDLED;
            }
        } else if (event.dwDebugEventCode == EXIT_PROCESS_DEBUG_EVENT) {
            printf("exit_process_code=%08lX\n", event.u.ExitProcess.dwExitCode);
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
    if (armed) put_byte(process, point, original);
    DebugActiveProcessStop(pid);
    CloseHandle(process);
    printf("hits=%d failed=%d\n", hits, failed);
    for (int index = 0; index <= 16; ++index)
        if (eax_small[index]) printf("eax_%d=%lu\n", index, eax_small[index]);
    if (eax_other) printf("eax_other=%lu\n", eax_other);
    if (point == 0x4492A0 || point == 0x44939E || point == 0x4454BA ||
        point == 0x4454EA || point == 0x44550A || point == 0x445568 ||
        point == 0x445584 || point == 0x448416 || point == 0x4484DA ||
        point == 0x4484EF || point == 0x44850A || point == 0x448D89)
        printf("type3_checks=%lu type3_rejected=%lu\n", type3_checks,
               type3_rejected);
    if (point == 0x44917B)
        printf("type3_target_checks=%lu over=%lu one_short=%lu two_short=%lu exact=%lu other=%lu\n",
               type3_checks, target_over, target_one_short, target_two_short,
               target_exact, target_other);
    if (point == 0x4621B4) printf("type3_unplaced=%lu\n", type3_unplaced);
    if (point == 0x4402BF)
        printf("type3_gate=%lu type3_gate_skipped=%lu\n", type3_gate,
               type3_gate_skipped);
    if (point == 0x44D5B9) {
        printf("type3_increment=%lu\n", type3_increment);
        for (int index = 0; index <= 16; ++index)
            if (type3_increment_small[index])
                printf("type3_amount_%d=%lu\n", index,
                       type3_increment_small[index]);
    }
    if (point == 0x415267)
        printf("serialized_complementarias=%lu\n", serialized_complementarias);
    if (point == 0x4620D5)
        printf("completion_quota_type3=%lu met=%lu short=%lu\n",
               completion_quota_type3, completion_quota_met,
               completion_quota_type3 - completion_quota_met);
    if (point == 0x4161F9)
        printf("duration_t_matrix_writes=%lu\n", duration_t_matrix_writes);
    if (point == 0x4161EA)
        printf("duration_m_matrix_writes=%lu\n", duration_m_matrix_writes);
    if (point == 0x44D357)
        printf("duration_matrix_reads=%lu value_2=%lu value_3=%lu other=%lu\n",
               duration_matrix_reads, duration_matrix_two,
               duration_matrix_three,
               duration_matrix_reads - duration_matrix_two - duration_matrix_three);
    if (point == 0x44D357)
        for (int index = 0; index < 5; ++index)
            if (duration_matrix_two_type[index])
                printf("duration_matrix_value_2_type_%d=%lu\n",
                       index, duration_matrix_two_type[index]);
    if (point == 0x44D357)
        for (int index = 0; index < 5; ++index)
            if (duration_matrix_three_type[index])
                printf("duration_matrix_value_3_type_%d=%lu\n",
                       index, duration_matrix_three_type[index]);
    return failed ? 1 : 0;
}
