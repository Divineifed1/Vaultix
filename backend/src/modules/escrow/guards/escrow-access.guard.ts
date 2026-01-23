import {
  Injectable,
  CanActivate,
  ExecutionContext,
  ForbiddenException,
  NotFoundException,
} from '@nestjs/common';
import { EscrowService } from '../services/escrow.service';

@Injectable()
export class EscrowAccessGuard implements CanActivate {
  constructor(private escrowService: EscrowService) {}

  async canActivate(context: ExecutionContext): Promise<boolean> {
    const request = context.switchToHttp().getRequest();
    const user = request.user;
    const escrowId = request.params.id;

    if (!user || !user.sub) {
      throw new ForbiddenException('User not authenticated');
    }

    if (!escrowId) {
      return true;
    }

    const escrow = await this.escrowService.findOne(escrowId);
    if (!escrow) {
      throw new NotFoundException('Escrow not found');
    }

    const isParty = await this.escrowService.isUserPartyToEscrow(
      escrowId,
      user.sub,
    );

    if (!isParty) {
      throw new ForbiddenException('You do not have access to this escrow');
    }

    request.escrow = escrow;
    return true;
  }
}
