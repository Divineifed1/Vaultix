import { Test, TestingModule } from '@nestjs/testing';
import { INestApplication, ValidationPipe } from '@nestjs/common';
import supertest from 'supertest';
import { AppModule } from '../src/app.module';
import { TypeOrmModule } from '@nestjs/typeorm';
import { User } from '../src/modules/user/entities/user.entity';
import { RefreshToken } from '../src/modules/user/entities/refresh-token.entity';
import { Escrow } from '../src/modules/escrow/entities/escrow.entity';
import { Party, PartyRole } from '../src/modules/escrow/entities/party.entity';
import { Condition } from '../src/modules/escrow/entities/condition.entity';
import { EscrowEvent } from '../src/modules/escrow/entities/escrow-event.entity';
import * as StellarSdk from 'stellar-sdk';

describe('Escrow (e2e)', () => {
  let app: INestApplication;
  let testKeypair: StellarSdk.Keypair;
  let testWalletAddress: string;
  let accessToken: string;
  let userId: string;

  let secondKeypair: StellarSdk.Keypair;
  let secondWalletAddress: string;
  let secondAccessToken: string;
  let secondUserId: string;

  beforeAll(async () => {
    testKeypair = StellarSdk.Keypair.random();
    testWalletAddress = testKeypair.publicKey();

    secondKeypair = StellarSdk.Keypair.random();
    secondWalletAddress = secondKeypair.publicKey();

    const moduleFixture: TestingModule = await Test.createTestingModule({
      imports: [
        AppModule,
        TypeOrmModule.forRoot({
          type: 'sqlite',
          database: ':memory:',
          entities: [User, RefreshToken, Escrow, Party, Condition, EscrowEvent],
          synchronize: true,
        }),
      ],
    }).compile();

    app = moduleFixture.createNestApplication();
    app.useGlobalPipes(new ValidationPipe({ transform: true }));
    await app.init();

    // Authenticate first user
    const challengeResponse = await supertest(app.getHttpServer())
      .post('/auth/challenge')
      .send({ walletAddress: testWalletAddress });

    const message = challengeResponse.body.message;
    const signature = testKeypair.sign(message).toString('hex');

    const verifyResponse = await supertest(app.getHttpServer())
      .post('/auth/verify')
      .send({
        walletAddress: testWalletAddress,
        signature: signature,
        publicKey: testWalletAddress,
      });

    accessToken = verifyResponse.body.accessToken;

    const meResponse = await supertest(app.getHttpServer())
      .get('/auth/me')
      .set('Authorization', `Bearer ${accessToken}`);
    userId = meResponse.body.id;

    // Authenticate second user
    const challenge2 = await supertest(app.getHttpServer())
      .post('/auth/challenge')
      .send({ walletAddress: secondWalletAddress });

    const message2 = challenge2.body.message;
    const signature2 = secondKeypair.sign(message2).toString('hex');

    const verify2 = await supertest(app.getHttpServer())
      .post('/auth/verify')
      .send({
        walletAddress: secondWalletAddress,
        signature: signature2,
        publicKey: secondWalletAddress,
      });

    secondAccessToken = verify2.body.accessToken;

    const me2 = await supertest(app.getHttpServer())
      .get('/auth/me')
      .set('Authorization', `Bearer ${secondAccessToken}`);
    secondUserId = me2.body.id;
  });

  afterAll(async () => {
    await app.close();
  });

  describe('POST /escrows', () => {
    it('should create an escrow', async () => {
      const response = await supertest(app.getHttpServer())
        .post('/escrows')
        .set('Authorization', `Bearer ${accessToken}`)
        .send({
          title: 'Test Escrow',
          description: 'Test description',
          amount: 100,
          asset: 'XLM',
          parties: [{ userId: secondUserId, role: PartyRole.SELLER }],
        })
        .expect(201);

      expect(response.body).toHaveProperty('id');
      expect(response.body.title).toBe('Test Escrow');
      expect(response.body.amount).toBe(100);
      expect(response.body.status).toBe('pending');
      expect(response.body.creatorId).toBe(userId);
    });

    it('should create an escrow with conditions', async () => {
      const response = await supertest(app.getHttpServer())
        .post('/escrows')
        .set('Authorization', `Bearer ${accessToken}`)
        .send({
          title: 'Escrow with Conditions',
          amount: 200,
          parties: [{ userId: secondUserId, role: PartyRole.SELLER }],
          conditions: [
            { description: 'Goods delivered', type: 'manual' },
            { description: 'Inspection passed', type: 'manual' },
          ],
        })
        .expect(201);

      expect(response.body.conditions).toHaveLength(2);
    });

    it('should return 400 for invalid data', async () => {
      await supertest(app.getHttpServer())
        .post('/escrows')
        .set('Authorization', `Bearer ${accessToken}`)
        .send({
          title: 'Test',
          // missing required fields
        })
        .expect(400);
    });

    it('should return 401 without auth token', async () => {
      await supertest(app.getHttpServer())
        .post('/escrows')
        .send({
          title: 'Test Escrow',
          amount: 100,
          parties: [{ userId: secondUserId, role: PartyRole.SELLER }],
        })
        .expect(401);
    });
  });

  describe('GET /escrows', () => {
    it('should return user escrows', async () => {
      const response = await supertest(app.getHttpServer())
        .get('/escrows')
        .set('Authorization', `Bearer ${accessToken}`)
        .expect(200);

      expect(response.body).toHaveProperty('data');
      expect(response.body).toHaveProperty('total');
      expect(response.body).toHaveProperty('page');
      expect(response.body).toHaveProperty('limit');
      expect(Array.isArray(response.body.data)).toBe(true);
    });

    it('should support pagination', async () => {
      const response = await supertest(app.getHttpServer())
        .get('/escrows?page=1&limit=5')
        .set('Authorization', `Bearer ${accessToken}`)
        .expect(200);

      expect(response.body.page).toBe(1);
      expect(response.body.limit).toBe(5);
    });

    it('should filter by status', async () => {
      const response = await supertest(app.getHttpServer())
        .get('/escrows?status=pending')
        .set('Authorization', `Bearer ${accessToken}`)
        .expect(200);

      response.body.data.forEach((escrow: any) => {
        expect(escrow.status).toBe('pending');
      });
    });

    it('should return 401 without auth token', async () => {
      await supertest(app.getHttpServer()).get('/escrows').expect(401);
    });
  });

  describe('GET /escrows/:id', () => {
    let escrowId: string;

    beforeAll(async () => {
      const response = await supertest(app.getHttpServer())
        .post('/escrows')
        .set('Authorization', `Bearer ${accessToken}`)
        .send({
          title: 'Get Test Escrow',
          amount: 50,
          parties: [{ userId: secondUserId, role: PartyRole.SELLER }],
        });
      escrowId = response.body.id;
    });

    it('should return escrow details for creator', async () => {
      const response = await supertest(app.getHttpServer())
        .get(`/escrows/${escrowId}`)
        .set('Authorization', `Bearer ${accessToken}`)
        .expect(200);

      expect(response.body.id).toBe(escrowId);
      expect(response.body.title).toBe('Get Test Escrow');
    });

    it('should return escrow details for party', async () => {
      const response = await supertest(app.getHttpServer())
        .get(`/escrows/${escrowId}`)
        .set('Authorization', `Bearer ${secondAccessToken}`)
        .expect(200);

      expect(response.body.id).toBe(escrowId);
    });

    it('should return 404 for non-existent escrow', async () => {
      await supertest(app.getHttpServer())
        .get('/escrows/non-existent-id')
        .set('Authorization', `Bearer ${accessToken}`)
        .expect(404);
    });
  });

  describe('PATCH /escrows/:id', () => {
    let escrowId: string;

    beforeEach(async () => {
      const response = await supertest(app.getHttpServer())
        .post('/escrows')
        .set('Authorization', `Bearer ${accessToken}`)
        .send({
          title: 'Update Test Escrow',
          amount: 75,
          parties: [{ userId: secondUserId, role: PartyRole.SELLER }],
        });
      escrowId = response.body.id;
    });

    it('should update escrow by creator', async () => {
      const response = await supertest(app.getHttpServer())
        .patch(`/escrows/${escrowId}`)
        .set('Authorization', `Bearer ${accessToken}`)
        .send({ title: 'Updated Title' })
        .expect(200);

      expect(response.body.title).toBe('Updated Title');
    });

    it('should return 403 when non-creator tries to update', async () => {
      await supertest(app.getHttpServer())
        .patch(`/escrows/${escrowId}`)
        .set('Authorization', `Bearer ${secondAccessToken}`)
        .send({ title: 'Unauthorized Update' })
        .expect(403);
    });
  });

  describe('POST /escrows/:id/cancel', () => {
    let escrowId: string;

    beforeEach(async () => {
      const response = await supertest(app.getHttpServer())
        .post('/escrows')
        .set('Authorization', `Bearer ${accessToken}`)
        .send({
          title: 'Cancel Test Escrow',
          amount: 25,
          parties: [{ userId: secondUserId, role: PartyRole.SELLER }],
        });
      escrowId = response.body.id;
    });

    it('should cancel escrow by creator', async () => {
      const response = await supertest(app.getHttpServer())
        .post(`/escrows/${escrowId}/cancel`)
        .set('Authorization', `Bearer ${accessToken}`)
        .send({ reason: 'Changed my mind' })
        .expect(201);

      expect(response.body.status).toBe('cancelled');
    });

    it('should return 403 when non-creator tries to cancel pending escrow', async () => {
      await supertest(app.getHttpServer())
        .post(`/escrows/${escrowId}/cancel`)
        .set('Authorization', `Bearer ${secondAccessToken}`)
        .send({ reason: 'Unauthorized cancel' })
        .expect(403);
    });

    it('should return 400 when trying to cancel already cancelled escrow', async () => {
      // First cancel
      await supertest(app.getHttpServer())
        .post(`/escrows/${escrowId}/cancel`)
        .set('Authorization', `Bearer ${accessToken}`)
        .send({});

      // Try to cancel again
      await supertest(app.getHttpServer())
        .post(`/escrows/${escrowId}/cancel`)
        .set('Authorization', `Bearer ${accessToken}`)
        .send({})
        .expect(400);
    });
  });
});
