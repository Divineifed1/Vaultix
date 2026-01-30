// Define types based on backend entities
export interface Party {
  id: string;
  userId: string;
  role: 'BUYER' | 'SELLER' | 'ARBITRATOR';
  status: 'PENDING' | 'ACCEPTED' | 'REJECTED';
  createdAt: string;
}

export interface Condition {
  id: string;
  description: string;
  type: string;
  metadata?: Record<string, any>;
}

export interface EscrowEvent {
  id: string;
  eventType: 'CREATED' | 'PARTY_ADDED' | 'PARTY_ACCEPTED' | 'PARTY_REJECTED' | 'FUNDED' | 'CONDITION_MET' | 'STATUS_CHANGED' | 'UPDATED' | 'CANCELLED' | 'COMPLETED' | 'DISPUTED';
  actorId?: string;
  data?: Record<string, any>;
  ipAddress?: string;
  createdAt: string;
}

export interface Escrow {
  id: string;
  title: string;
  description?: string;
  amount: number;
  asset: string;
  status: 'PENDING' | 'ACTIVE' | 'COMPLETED' | 'CANCELLED' | 'DISPUTED';
  type: 'STANDARD' | 'MILESTONE' | 'TIMED';
  creatorId: string;
  expiresAt?: string;
  isActive: boolean;
  creator: {
    id: string;
    walletAddress?: string;
  };
  parties: Party[];
  conditions: Condition[];
  events: EscrowEvent[];
  createdAt: string;
  updatedAt: string;
}

export interface UseEscrowReturn {
  escrow: Escrow | null;
  loading: boolean;
  error: string | null;
}

export interface WalletHookReturn {
  connected: boolean;
  publicKey: string | null;
  connect: () => void;
}