/** Thin wrapper that sources the wallet from `@solana/wallet-adapter-react`. */
import { useWallet } from '@solana/wallet-adapter-react';
import {
  useInvokeSkillWithWallet,
  type UseInvokeSkillOptions,
  type UseInvokeSkillResult,
  type UseInvokeSkillWallet,
} from './useInvokeSkill.js';

export function useInvokeSkill(options: UseInvokeSkillOptions = {}): UseInvokeSkillResult {
  const wallet = useWallet() as unknown as UseInvokeSkillWallet;
  return useInvokeSkillWithWallet(wallet, options);
}
